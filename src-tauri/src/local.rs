use std::time::Duration;

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{async_runtime, AppHandle, Manager, Runtime};
use tauri_plugin_pinia::ManagerExt;
use tauri_specta::Event;
use tokio::{select, sync::mpsc, time};
use tpower::{
    ffi::smc::{SMCConnection, SMCPowerData, SMCReadSensor},
    provider::{get_mac_ioreg, NormalizedData, NormalizedResource},
};

use crate::event::{PowerUpdatedEvent, PreferenceEvent, StatusBarItem, WindowLoadedEvent};

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
    pub data: NormalizedResource,
}

fn emit_local_power_tick<R: Runtime>(
    app: &AppHandle<R>,
    smc: &SMCPowerData,
    status_bar_item: &StatusBarItem,
    show_charging: bool,
) {
    if let Err(e) =
        PowerUpdatedEvent::new_with(smc, status_bar_item, show_charging).emit(app)
    {
        log::warn!("failed to emit PowerUpdatedEvent: {e}");
    }

    // Prefer full IORegistry + SMC data; if IORegistry parse fails (e.g. macOS
    // schema changes), still publish SMC-only numbers so the tray keeps working.
    let data = match get_mac_ioreg() {
        Ok(ioreg) => (&ioreg, smc).into(),
        Err(e) => {
            log::warn!("failed to read local IORegistry, falling back to SMC only: {e}");
            NormalizedResource {
                is_local: true,
                is_charging: smc.is_charging(),
                time_remain: Duration::from_secs_f32(
                    60.0 * if smc.is_charging() {
                        smc.time_to_full
                    } else {
                        smc.time_to_empty
                    },
                ),
                data: NormalizedData {
                    system_in: smc.delivery_rate,
                    system_load: smc.system_total,
                    battery_power: smc.battery_rate.max(smc.delivery_rate - smc.system_total),
                    adapter_power: smc.delivery_rate,
                    brightness_power: smc.brightness,
                    heatpipe_power: smc.heatpipe,
                    temperature: smc.temperature,
                    ..Default::default()
                },
                ..Default::default()
            }
        }
    };

    if let Err(e) = (PowerTickEvent { data }).emit(app) {
        log::warn!("failed to emit PowerTickEvent: {e}");
    }
}

pub fn start_sender<R: Runtime>(
    app: &impl Manager<R>,
    mut rx: mpsc::Receiver<SenderMessage>,
) -> async_runtime::JoinHandle<()> {
    let app = app.app_handle().clone();

    let mut smc_conn = match SMCConnection::new("AppleSMC") {
        Ok(conn) => conn,
        Err(e) => {
            log::error!("failed to open AppleSMC connection (kern={e}); local power sender disabled");
            // Keep the channel drained so senders never hang, but do not panic.
            return async_runtime::spawn(async move {
                while rx.recv().await.is_some() {}
            });
        }
    };

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
                    emit_local_power_tick(&app, &smc, &status_bar_item, show_charging);
                }
                Some(msg) = rx.recv() => match msg {
                    SenderMessage::ImmediateSend => {
                        let smc = smc_conn.read_sensor();
                        emit_local_power_tick(&app, &smc, &status_bar_item, show_charging);
                    },
                    SenderMessage::ChangeInterval(interval) => {
                        timer = time::interval(if interval < Duration::from_millis(500) {
                            log::warn!("interval is too small, set to 500ms");
                            Duration::from_millis(500)
                        } else {
                            interval
                        });
                    },
                    SenderMessage::ChangeStatusBarItem(item) => {
                        status_bar_item = item;
                        let smc = smc_conn.read_sensor();
                        if let Err(e) = PowerUpdatedEvent::new_with(&smc, &status_bar_item, show_charging)
                            .emit(&app)
                        {
                            log::warn!("failed to emit PowerUpdatedEvent: {e}");
                        }
                    },
                    SenderMessage::StatusBarShowCharging(show) => {
                        show_charging = show;
                        let smc = smc_conn.read_sensor();
                        if let Err(e) = PowerUpdatedEvent::new_with(&smc, &status_bar_item, show_charging)
                            .emit(&app)
                        {
                            log::warn!("failed to emit PowerUpdatedEvent: {e}");
                        }
                    }
                }
            }
        }
    })
}

pub fn setup_sender_with_events<R: Runtime>(app: &impl Manager<R>) {
    let app = app.app_handle();
    let (sender_tx, rx) = mpsc::channel(10);
    start_sender(app, rx);

    // send an immediate update when the main window is loaded
    let tx = sender_tx.clone();
    WindowLoadedEvent::listen(app, move |_| {
        let tx = tx.clone();
        async_runtime::spawn(async move {
            tx.send(SenderMessage::ImmediateSend).await.unwrap();
        });
    });

    let tx = sender_tx.clone();
    PreferenceEvent::listen(app, move |event| {
        if let Some(msg) = match event.payload {
            PreferenceEvent::UpdateInterval(interval) => Some(SenderMessage::ChangeInterval(
                Duration::from_millis(interval.into()),
            )),
            PreferenceEvent::StatusBarItem(item) => Some(SenderMessage::ChangeStatusBarItem(item)),
            PreferenceEvent::StatusBarShowCharging(show) => {
                Some(SenderMessage::StatusBarShowCharging(show))
            }
            PreferenceEvent::Language(_) => {
                // No need to send, perform some menu refreshing
                None
            }
            _ => None,
        } {
            let tx = tx.clone();
            async_runtime::spawn(async move {
                tx.send(msg).await.unwrap();
            });
        }
    });
}
