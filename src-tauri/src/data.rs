use std::{
    collections::HashMap,
    ffi::c_void,
    mem::{self, MaybeUninit},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{async_runtime, AppHandle, Manager, Runtime};
use tauri_plugin_pinia::ManagerExt;
use tauri_specta::Event;
use tokio::{select, sync::mpsc, task::spawn_blocking, time};
use tpower::{
    de::IORegistry,
    ffi::{
        core_foundation::runloop::CFRunLoopRun,
        smc::{SMCConnection, SMCPowerData, SMCReadSensor},
        wrapper::{Device, ServiceConnection},
        AMDeviceNotificationCallbackInfo, AMDeviceNotificationSubscribe, Action,
    },
    provider::{get_mac_ioreg, remote::get_device_ioreg},
};

use crate::event::{DeviceEvent, DevicePowerTickEvent, PowerUpdatedEvent, StatusBarItem};

pub enum SenderMessage {
    ImmediateSend,
    ChangeInterval(Duration),
    ChangeStatusBarItem(StatusBarItem),
    StatusBarShowCharging(bool),
}

pub fn status_bar_text(
    smc: &SMCPowerData,
    status_bar_item: &StatusBarItem,
    show_charging: bool,
) -> f32 {
    if smc.is_charging() && show_charging {
        return smc.delivery_rate;
    }
    match status_bar_item {
        StatusBarItem::System => smc.system_total,
        StatusBarItem::Screen => smc.brightness,
        StatusBarItem::Heatpipe => smc.heatpipe,
    }
}

impl PowerUpdatedEvent {
    pub fn new(value: f32) -> Self {
        Self(format!("{:.1} w", value))
    }

    pub fn new_with(
        smc: &SMCPowerData,
        status_bar_item: &StatusBarItem,
        show_charging: bool,
    ) -> Self {
        Self::new(status_bar_text(smc, status_bar_item, show_charging))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Event, Type)]
#[serde(rename_all = "camelCase")]
pub struct PowerTickEvent {
    pub io: IORegistry,
    pub smc: SMCPowerData,
}

pub fn start_sender<R: Runtime>(
    app: &impl Manager<R>,
    mut rx: mpsc::Receiver<SenderMessage>,
) -> async_runtime::JoinHandle<()> {
    let app = app.app_handle().clone();
    let mut smc_conn = SMCConnection::new("AppleSMC").unwrap();

    let mut timer = time::interval(Duration::from_millis(
        app.pinia()
            .try_get::<u64>("preference", "updateInterval")
            .unwrap_or(2000),
    ));
    let mut status_bar_item = app
        .pinia()
        .try_get::<StatusBarItem>("preference", "statusBarItem")
        .unwrap_or(StatusBarItem::System);
    let mut show_charging = app
        .pinia()
        .try_get::<bool>("preference", "showCharging")
        .unwrap_or(true);

    async_runtime::spawn(async move {
        loop {
            select! {
                _ = timer.tick() => {
                    let smc = smc_conn.read_sensor();
                    PowerUpdatedEvent::new_with(&smc, &status_bar_item, show_charging)
                        .emit(&app)
                        .unwrap();
                    PowerTickEvent {
                        io: get_mac_ioreg().unwrap(),
                        smc
                    }.emit(&app).unwrap();
                }
                Some(msg) = rx.recv() => {
                    match msg {
                        SenderMessage::ImmediateSend => {
                            let smc = smc_conn.read_sensor();
                            PowerUpdatedEvent::new_with(&smc, &status_bar_item, show_charging)
                                .emit(&app)
                                .unwrap();
                            PowerTickEvent {
                                io: get_mac_ioreg().unwrap(),
                                smc
                            }.emit(&app).unwrap();
                        },
                        SenderMessage::ChangeInterval(interval) => {
                            timer = time::interval(if interval < Duration::from_millis(500) {
                                Duration::from_millis(500)
                            } else {
                                interval
                            });
                        },
                        SenderMessage::ChangeStatusBarItem(item) => {
                            status_bar_item = item;
                            PowerUpdatedEvent::new_with(&smc_conn.read_sensor(), &status_bar_item, show_charging)
                                .emit(&app)
                                .unwrap();
                        },
                        SenderMessage::StatusBarShowCharging(show) => {
                            show_charging = show;
                            PowerUpdatedEvent::new_with(&smc_conn.read_sensor(), &status_bar_item, show_charging)
                                .emit(&app)
                                .unwrap();
                        }
                    }
                }
            }
        }
    })
}

#[derive(Debug)]
pub struct DeviceMessage {
    device: Device,
    action: Action,
}

pub fn start_device_listener() -> mpsc::Receiver<DeviceMessage> {
    let (tx, rx) = mpsc::channel::<DeviceMessage>(10);

    extern "C" fn callback(info: *const AMDeviceNotificationCallbackInfo, context: *mut c_void) {
        let tx = unsafe { Box::from_raw(context as *mut mpsc::Sender<DeviceMessage>) };
        let info = unsafe { *&*info };
        let device = unsafe { Device::new(info.device) };

        async_runtime::spawn(async move {
            tx.send(DeviceMessage {
                device,
                action: info.action,
            })
            .await
            .unwrap();
            mem::forget(tx);
        });
    }

    spawn_blocking(move || {
        let boxed = Box::new(tx);
        let mut not = MaybeUninit::uninit();
        unsafe {
            AMDeviceNotificationSubscribe(
                callback,
                0,
                0,
                Box::into_raw(boxed) as *mut _,
                not.as_mut_ptr(),
            )
        };
        unsafe { CFRunLoopRun() };
    });

    rx
}

pub fn start_device_sender(handle: AppHandle) -> async_runtime::JoinHandle<()> {
    let mut rx = start_device_listener();
    let mut timer = time::interval(Duration::from_millis(1000));

    let mut devices: HashMap<Device, ServiceConnection> = HashMap::new();

    async_runtime::spawn(async move {
        loop {
            select! {
                _ = timer.tick() => {
                    for (device, conn) in devices.iter() {
                        match get_device_ioreg(conn) {
                            Ok(res) => DevicePowerTickEvent {
                                io: res,
                                udid: device.udid.clone(),
                            }.emit(&handle).unwrap(),
                            Err(err) => {
                                log::error!("Failed to get IORegistry: {err}");
                            }
                        }
                    }
                }
                Some(DeviceMessage { device, action }) = rx.recv() => {

                    match action {
                        Action::Attached => {
                            // unwrap pair
                            device.prepare_device().unwrap();
                            let conn = device.start_service("com.apple.mobile.diagnostics_relay");

                            DeviceEvent {
                                udid: device.udid.clone(),
                                // must call `device.name()` after `device.prepare_device()`
                                // or name will be empty causing panic
                                name: device.name(),
                                interface: device.interface_type,
                                action,
                            }.emit(&handle).unwrap();

                            devices.insert(device, conn);
                        },
                        Action::Detached => {
                            log::info!("Device detached: {}", device.udid);
                            DeviceEvent {
                                udid: device.udid.clone(),
                                name: String::new(),
                                interface: device.interface_type,
                                action,
                            }.emit(&handle).unwrap();
                            devices.remove(&device);
                        },
                        _ => ()
                    }
                }
            }
        }
    })
}
