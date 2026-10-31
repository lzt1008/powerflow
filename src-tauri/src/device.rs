use std::{
    collections::{HashMap, HashSet},
    ffi::c_void,
    mem::{self, MaybeUninit},
    sync::{Arc, RwLock},
    time::Duration,
};

use derive_more::derive::Deref;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{async_runtime, AppHandle, Manager};
use tauri_specta::Event;
use tokio::{select, sync::mpsc, task::spawn_blocking, time};
use tpower::{
    ffi::{
        core_foundation::runloop::CFRunLoopRun,
        wrapper::{Device, ServiceConnection},
        AMDeviceNotificationCallbackInfo, AMDeviceNotificationSubscribe, Action, InterfaceType,
    },
    provider::{remote::get_device_ioreg, NormalizedResource},
};

use crate::event::DeviceEvent;

#[derive(Default, Deref)]
pub struct DeviceState(RwLock<HashMap<String, (String, HashSet<InterfaceType>)>>);

#[derive(Serialize, Deserialize, Debug, Clone, Event, Type)]
#[serde(rename_all = "camelCase")]
pub struct DevicePowerTickEvent {
    pub udid: String,
    pub data: NormalizedResource,
}

#[derive(Debug)]
pub struct DeviceMessage {
    device: Device,
    action: Action,
}

pub fn start_device_listener() -> mpsc::Receiver<DeviceMessage> {
    let (tx, rx) = mpsc::channel::<DeviceMessage>(10);

    extern "C" fn callback(info: *const AMDeviceNotificationCallbackInfo, context: *mut c_void) {
        let tx = unsafe { &*(context as *mut mpsc::Sender<DeviceMessage>) };
        let info = unsafe { *info };
        let device = unsafe { Device::new(info.device) };

        let tx = tx.clone();

        async_runtime::spawn(async move {
            if tx
                .send(DeviceMessage {
                    device,
                    action: info.action,
                })
                .await
                .is_err()
            {
                log::warn!("Device listener receiver gone, dropping device message");
            }
            mem::forget(tx);
        });
    }

    spawn_blocking(move || {
        let boxed = Arc::new(tx);
        let mut not = MaybeUninit::uninit();
        unsafe {
            AMDeviceNotificationSubscribe(
                callback,
                0,
                0,
                Arc::as_ptr(&boxed) as *mut _,
                not.as_mut_ptr(),
            )
        };
        unsafe { CFRunLoopRun() };
    });

    rx
}

pub fn start_device_sender(handle: AppHandle) -> async_runtime::JoinHandle<()> {
    let mut rx = start_device_listener();
    let mut timer = time::interval(Duration::from_millis(2000));

    let mut devices: HashMap<Device, ServiceConnection> = HashMap::new();

    async_runtime::spawn(async move {
        loop {
            select! {
                _ = timer.tick() => {
                    for (device, conn) in devices.iter() {
                        match get_device_ioreg(conn) {
                            Ok(res) => {
                                if let Err(err) = (DevicePowerTickEvent {
                                    udid: device.udid.clone(),
                                    data: NormalizedResource::from(&res),
                                })
                                .emit(&handle)
                                {
                                    log::warn!("Failed to emit device power tick: {err}");
                                }
                            }
                            Err(err) => {
                                log::error!("Failed to get IORegistry: {err}");
                            }
                        }
                    }
                }
                Some(DeviceMessage { device, action }) = rx.recv() => {
                    match action {
                        Action::Attached => {
                            if let Err(err) = device.prepare_device() {
                                log::error!("Failed to prepare device {}: {err}", device.udid);
                            } else {
                                let conn =
                                    device.start_service("com.apple.mobile.diagnostics_relay");

                                if let Err(err) = (DeviceEvent {
                                    udid: device.udid.clone(),
                                    // must call `device.name()` after `device.prepare_device()`
                                    // or name will be empty
                                    name: device.name(),
                                    interface: device.interface_type,
                                    action,
                                })
                                .emit(&handle)
                                {
                                    log::warn!("Failed to emit device event: {err}");
                                }

                                devices.insert(device, conn);
                            }
                        },
                        Action::Detached => {
                            log::debug!("Device detached: {}", device.udid);
                            if let Err(err) = (DeviceEvent {
                                udid: device.udid.clone(),
                                name: String::new(),
                                interface: device.interface_type,
                                action,
                            })
                            .emit(&handle)
                            {
                                log::warn!("Failed to emit device event: {err}");
                            }
                            devices.remove(&device);
                        },
                        _ => ()
                    }
                }
            }
        }
    })
}

pub fn setup_device_listener(app: AppHandle) {
    DeviceEvent::listen(&app.clone(), move |event| {
        let event = event.payload;
        let app_state = app.state::<DeviceState>();

        use scopefn::Run;
        app_state
            .write()
            .unwrap()
            .entry(event.udid.clone())
            .or_insert_with(|| (event.name, HashSet::new()))
            .run(|e| match event.action {
                Action::Attached => {
                    e.1.insert(event.interface);
                }
                Action::Detached => {
                    e.1.remove(&event.interface);
                }
                _ => (),
            });
    });
}
