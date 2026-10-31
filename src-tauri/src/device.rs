use std::{
    collections::{HashMap, HashSet},
    ffi::c_void,
    ptr::null_mut,
    sync::RwLock,
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
        AMDeviceNotificationCallbackInfo, AMDeviceNotificationSubscribe,
        AMDeviceNotificationUnsubscribe, Action, InterfaceType,
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

pub fn start_device_listener() -> mpsc::UnboundedReceiver<DeviceMessage> {
    let (tx, rx) = mpsc::unbounded_channel::<DeviceMessage>();

    extern "C" fn callback(info: *const AMDeviceNotificationCallbackInfo, context: *mut c_void) {
        let tx = unsafe { &*(context as *const mpsc::UnboundedSender<DeviceMessage>) };
        let info = unsafe { &*info };
        if !matches!(info.action, Action::Attached | Action::Detached) {
            return;
        }
        let device = unsafe { Device::new(info.device) };
        // Forward synchronously so detach/attach events keep their original order.
        if tx
            .send(DeviceMessage {
                device,
                action: info.action,
            })
            .is_err()
        {
            log::warn!("device listener receiver closed");
        }
    }

    spawn_blocking(move || {
        let sender = Box::new(tx);
        let mut subscription = null_mut();
        let result = unsafe {
            AMDeviceNotificationSubscribe(
                callback,
                0,
                0,
                (&*sender as *const mpsc::UnboundedSender<DeviceMessage>)
                    .cast_mut()
                    .cast(),
                &mut subscription,
            )
        };
        if result != 0 {
            log::error!("failed to subscribe to device notifications: {result}");
            return;
        }
        unsafe { CFRunLoopRun() };
        unsafe { AMDeviceNotificationUnsubscribe(subscription.cast()) };
    });
    rx
}

struct DeviceConnection {
    // Drop the diagnostics service before closing its device session.
    service: Option<ServiceConnection>,
    device: Device,
}

fn emit_device_state(handle: &AppHandle, device: &Device, action: Action) {
    let event = DeviceEvent {
        udid: device.udid.clone(),
        name: if action == Action::Attached {
            device.name()
        } else {
            String::new()
        },
        interface: device.interface_type,
        action,
    };
    if let Err(error) = event.emit(handle) {
        log::warn!("failed to emit device state: {error}");
    }
}

pub fn start_device_sender(handle: AppHandle) -> async_runtime::JoinHandle<()> {
    let mut rx = start_device_listener();
    let mut timer = time::interval(Duration::from_millis(2000));
    let mut devices: HashMap<(String, InterfaceType), DeviceConnection> = HashMap::new();

    async_runtime::spawn(async move {
        loop {
            select! {
                _ = timer.tick() => {
                    for entry in devices.values_mut() {
                        if entry.service.is_none() {
                            match entry.device.prepare_device().and_then(|()| {
                                entry.device.start_service("com.apple.mobile.diagnostics_relay")
                            }) {
                                Ok(service) => {
                                    entry.service = Some(service);
                                    log::info!("device connected via {:?}", entry.device.interface_type);
                                    emit_device_state(&handle, &entry.device, Action::Attached);
                                }
                                Err(error) => {
                                    entry.device.disconnect();
                                    log::warn!("failed to connect {:?} device, retrying: {error}", entry.device.interface_type);
                                    continue;
                                }
                            }
                        }
                        let Some(service) = entry.service.as_ref() else { continue };
                        match get_device_ioreg(service) {
                            Ok(res) => {
                                if let Err(error) = (DevicePowerTickEvent {
                                    udid: entry.device.udid.clone(),
                                    data: NormalizedResource::from(&res),
                                }).emit(&handle) {
                                    log::warn!("failed to emit device power: {error}");
                                }
                            }
                            Err(error) => {
                                log::warn!("failed to read {:?} device, reconnecting: {error}", entry.device.interface_type);
                                entry.service.take();
                                entry.device.disconnect();
                                emit_device_state(&handle, &entry.device, Action::Detached);
                            }
                        }
                    }
                }
                message = rx.recv() => {
                    let Some(DeviceMessage { device, action }) = message else { break };
                    let key = (device.udid.clone(), device.interface_type);
                    match action {
                        Action::Attached => {
                            devices.entry(key).or_insert(DeviceConnection { service: None, device });
                        }
                        Action::Detached => {
                            devices.remove(&key);
                            log::info!("device disconnected via {:?}", device.interface_type);
                            emit_device_state(&handle, &device, Action::Detached);
                        }
                        _ => {}
                    }
                }
            }
        }
    })
}

impl DeviceState {
    fn update(&self, event: DeviceEvent) {
        let mut devices = self.write().unwrap();
        match event.action {
            Action::Attached => {
                let entry = devices.entry(event.udid).or_default();
                if !event.name.is_empty() {
                    entry.0 = event.name;
                }
                entry.1.insert(event.interface);
            }
            Action::Detached => {
                if let Some(entry) = devices.get_mut(&event.udid) {
                    entry.1.remove(&event.interface);
                }
            }
            _ => {}
        }
    }
}

pub fn setup_device_listener(app: AppHandle) {
    DeviceEvent::listen(&app.clone(), move |event| {
        app.state::<DeviceState>().update(event.payload);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(interface: InterfaceType, action: Action) -> DeviceEvent {
        DeviceEvent {
            udid: "phone".into(),
            name: "iPhone".into(),
            interface,
            action,
        }
    }

    #[test]
    fn unplugging_usb_keeps_wifi_and_removes_the_stale_usb_label() {
        let state = DeviceState::default();
        state.update(event(InterfaceType::USB, Action::Attached));
        state.update(event(InterfaceType::WiFi, Action::Attached));
        state.update(event(InterfaceType::USB, Action::Detached));
        {
            let devices = state.read().unwrap();
            assert_eq!(devices["phone"].1, HashSet::from([InterfaceType::WiFi]));
        }
        state.update(event(InterfaceType::WiFi, Action::Detached));
        assert!(state.read().unwrap()["phone"].1.is_empty());
    }

    #[test]
    fn detach_before_attach_does_not_create_a_phantom_device() {
        let state = DeviceState::default();
        state.update(event(InterfaceType::USB, Action::Detached));
        assert!(state.read().unwrap().is_empty());
    }
}
