use std::time::Duration;

use tauri::{async_runtime, Manager, Runtime};
use tauri_plugin_pinia::ManagerExt;
use tauri_specta::Event;
use tokio::{select, sync::mpsc};
use tpower::{
    ffi::smc::{SMCConnection, SMCPowerData, SMCReadSensor},
    provider::get_mac_ioreg,
};

use crate::event::{PowerTickEvent, PowerUpdatedEvent, StatusBarItem};

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

pub fn start_sender<R: Runtime>(
    app: &impl Manager<R>,
    mut rx: mpsc::Receiver<SenderMessage>,
) -> async_runtime::JoinHandle<()> {
    let app = app.app_handle().clone();
    let mut smc_conn = SMCConnection::new("AppleSMC").unwrap();

    let mut timer = tokio::time::interval(Duration::from_millis(
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
                            timer = tokio::time::interval(if interval < Duration::from_millis(100) {
                                // log::warn!("minimum interval is 100ms");
                                Duration::from_millis(100)
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
