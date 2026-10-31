use std::{
    collections::{HashMap, HashSet},
    ffi::c_void,
    mem::MaybeUninit,
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

/// Active remote connections keyed by UDID — at most one tick source per device
/// so USB+WiFi dual attach does not double-feed history.
struct ConnectedDevice {
    device: Device,
    conn: ServiceConnection,
}

pub fn start_device_listener() -> mpsc::UnboundedReceiver<DeviceMessage> {
    let (tx, rx) = mpsc::unbounded_channel::<DeviceMessage>();

    extern "C" fn callback(info: *const AMDeviceNotificationCallbackInfo, context: *mut c_void) {
        let tx = unsafe { &*(context as *mut mpsc::UnboundedSender<DeviceMessage>) };
        let info = unsafe { *info };
        let device = unsafe { Device::new(info.device, matches!(info.action, Action::Attached)) };

        if let Err(err) = tx.send(DeviceMessage {
            device,
            action: info.action,
        }) {
            log::error!("Failed to send device message: {err}");
        }
    }

    spawn_blocking(move || {
        let boxed = Arc::new(tx);
        let mut not = MaybeUninit::uninit();
        let result = unsafe {
            AMDeviceNotificationSubscribe(
                callback,
                0,
                0,
                Arc::as_ptr(&boxed) as *mut _,
                not.as_mut_ptr(),
            )
        };
        if result != 0 {
            log::error!("AMDeviceNotificationSubscribe failed: {result}");
            return;
        }
        // Keep the Arc alive for the lifetime of the run loop.
        let _keep = boxed;
        unsafe { CFRunLoopRun() };
    });

    rx
}

fn preferred_connection(
    connections: &HashMap<InterfaceType, ConnectedDevice>,
) -> Option<&ConnectedDevice> {
    preferred_interface(connections.keys().copied())
        .and_then(|interface| connections.get(&interface))
}

fn preferred_interface(interfaces: impl Iterator<Item = InterfaceType>) -> Option<InterfaceType> {
    let interfaces = interfaces.collect::<HashSet<_>>();
    [InterfaceType::USB, InterfaceType::WiFi]
        .into_iter()
        .find(|interface| interfaces.contains(interface))
        .or_else(|| interfaces.into_iter().next())
}

pub fn start_device_sender(handle: AppHandle) -> async_runtime::JoinHandle<()> {
    let mut rx = start_device_listener();
    let mut timer = time::interval(Duration::from_millis(2000));
    timer.set_missed_tick_behavior(time::MissedTickBehavior::Delay);

    // Keep one prepared connection per interface. Poll USB preferentially,
    // while retaining Wi-Fi as an immediate fallback.
    let mut devices: HashMap<String, HashMap<InterfaceType, ConnectedDevice>> = HashMap::new();

    async_runtime::spawn(async move {
        loop {
            select! {
                _ = timer.tick() => {
                    for connections in devices.values() {
                        let Some(connected) = preferred_connection(connections) else {
                            continue;
                        };
                        match get_device_ioreg(&connected.conn) {
                            Ok(res) => {
                                if let Err(err) = (DevicePowerTickEvent {
                                    udid: connected.device.udid.clone(),
                                    data: NormalizedResource::from(&res),
                                }).emit(&handle) {
                                    log::error!("Failed to emit DevicePowerTickEvent: {err}");
                                }
                            }
                            Err(err) => {
                                log::error!("Failed to get IORegistry: {err}");
                            }
                        }
                    }
                }
                Some(DeviceMessage { mut device, action }) = rx.recv() => {
                    match action {
                        Action::Attached => {
                            let udid = device.udid.clone();
                            if udid.is_empty() {
                                log::warn!("Ignoring attached device with empty UDID");
                                continue;
                            }

                            let interface = device.interface_type;
                            if devices
                                .get(&udid)
                                .is_some_and(|connections| connections.contains_key(&interface))
                            {
                                log::debug!("Ignoring duplicate attach for {udid} via {interface:?}");
                                continue;
                            }

                            if let Err(err) = device.prepare_device() {
                                log::error!("Failed to prepare device {udid}: {err}");
                                continue;
                            }

                            let name_after_prepare = device.name();

                            let conn = match device.start_service("com.apple.mobile.diagnostics_relay") {
                                Ok(conn) => conn,
                                Err(err) => {
                                    log::error!("Failed to start diagnostics_relay on {udid}: {err}");
                                    continue;
                                }
                            };

                            if let Err(err) = (DeviceEvent {
                                udid: udid.clone(),
                                // must call `device.name()` after `device.prepare_device()`
                                name: name_after_prepare,
                                interface,
                                action,
                            }).emit(&handle) {
                                log::error!("Failed to emit DeviceEvent: {err}");
                            }

                            devices
                                .entry(udid)
                                .or_default()
                                .insert(interface, ConnectedDevice { device, conn });
                        },
                        Action::Detached => {
                            log::debug!("Device detached: {}", device.udid);
                            if let Err(err) = (DeviceEvent {
                                udid: device.udid.clone(),
                                name: String::new(),
                                interface: device.interface_type,
                                action,
                            }).emit(&handle) {
                                log::error!("Failed to emit DeviceEvent: {err}");
                            }

                            if let Some(connections) = devices.get_mut(&device.udid) {
                                connections.remove(&device.interface_type);
                                if connections.is_empty() {
                                    devices.remove(&device.udid);
                                }
                            }
                            // Any remaining interface is selected on the next tick.
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
        let mut guard = match app_state.write() {
            Ok(g) => g,
            Err(e) => {
                log::error!("DeviceState lock poisoned: {e}");
                return;
            }
        };
        guard
            .entry(event.udid.clone())
            .or_insert_with(|| (event.name.clone(), HashSet::new()))
            .run(|e| match event.action {
                Action::Attached => {
                    if !event.name.is_empty() {
                        e.0.clone_from(&event.name);
                    }
                    e.1.insert(event.interface);
                }
                Action::Detached => {
                    e.1.remove(&event.interface);
                    if e.1.is_empty() {
                        // Keep the map entry so name lookup still works briefly;
                        // empty interface set is the offline signal for the UI.
                    }
                }
                _ => (),
            });
        let should_cleanup = guard
            .get(&event.udid)
            .is_some_and(|(_, interfaces)| interfaces.is_empty());
        drop(guard);

        if should_cleanup {
            let app = app.clone();
            let udid = event.udid;
            async_runtime::spawn(async move {
                time::sleep(Duration::from_secs(300)).await;
                if let Ok(mut state) = app.state::<DeviceState>().write() {
                    if state
                        .get(&udid)
                        .is_some_and(|(_, interfaces)| interfaces.is_empty())
                    {
                        state.remove(&udid);
                    }
                }
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usb_is_preferred_and_wifi_is_the_fallback() {
        assert_eq!(
            preferred_interface([InterfaceType::WiFi, InterfaceType::USB].into_iter()),
            Some(InterfaceType::USB)
        );
        assert_eq!(
            preferred_interface([InterfaceType::WiFi].into_iter()),
            Some(InterfaceType::WiFi)
        );
        assert_eq!(preferred_interface(std::iter::empty()), None);
    }
}
