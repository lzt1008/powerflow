use std::time::Duration;

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{async_runtime, AppHandle, Manager, Runtime};
use tauri_plugin_pinia::ManagerExt;
use tauri_specta::Event;
use tokio::{
    select,
    sync::mpsc,
    time::{self, Instant},
};
use tpower::{
    ffi::smc::{SMCConnection, SMCPowerData, SMCReadSensor},
    provider::{get_mac_ioreg, NormalizedResource},
};

use crate::event::{PowerUpdatedEvent, PreferenceEvent, StatusBarItem, WindowLoadedEvent};

pub enum SenderMessage {
    ImmediateSend,
    ChangeInterval(Duration),
    ChangeStatusBarItem(StatusBarItem),
    StatusBarShowCharging(bool),
}

/// Minimum (500 ms) and maximum (60 s) bounds for the power-tick interval.
///
/// Stale preference files (e.g. `updateInterval: 86400000` persisted by
/// earlier builds) would otherwise stall the chart for up to a day.
const MIN_INTERVAL_MS: u64 = 500;
const MAX_INTERVAL_MS: u64 = 60_000;

fn sanitize_interval_ms(ms: u64) -> Duration {
    if ms < MIN_INTERVAL_MS {
        log::warn!("updateInterval {ms}ms too small, clamped to {MIN_INTERVAL_MS}ms");
        Duration::from_millis(MIN_INTERVAL_MS)
    } else if ms > MAX_INTERVAL_MS {
        log::warn!("updateInterval {ms}ms too large, clamped to {MAX_INTERVAL_MS}ms");
        Duration::from_millis(MAX_INTERVAL_MS)
    } else {
        Duration::from_millis(ms)
    }
}

fn sanitize_interval(ms: u64) -> Duration {
    sanitize_interval_ms(ms)
}

fn make_interval(period: Duration) -> time::Interval {
    // Avoid the immediate first tick that `interval()` fires on creation.
    let mut timer = time::interval_at(Instant::now() + period, period);
    timer.set_missed_tick_behavior(time::MissedTickBehavior::Delay);
    timer
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

fn emit_power_sample<R: Runtime>(
    app: &AppHandle<R>,
    smc_conn: &mut Option<SMCConnection>,
    next_smc_retry: &mut Instant,
    status_bar_item: &StatusBarItem,
    show_charging: bool,
) {
    if smc_conn.is_none() && Instant::now() >= *next_smc_retry {
        match SMCConnection::new("AppleSMC") {
            Ok(connection) => {
                log::info!("AppleSMC connection restored");
                *smc_conn = Some(connection);
            }
            Err(error) => {
                log::warn!("AppleSMC unavailable ({error}); retrying in 30 seconds");
                *next_smc_retry = Instant::now() + Duration::from_secs(30);
            }
        }
    }

    let smc = smc_conn.as_mut().map(SMCReadSensor::read_sensor);
    match get_mac_ioreg() {
        Ok(ioreg) => {
            let data = smc.as_ref().map_or_else(
                || NormalizedResource::local_from_ioreg(&ioreg),
                |smc| NormalizedResource::from((&ioreg, smc)),
            );
            let bar = if data.is_charging && show_charging {
                data.adapter_power
            } else {
                match status_bar_item {
                    StatusBarItem::System => data.system_load,
                    StatusBarItem::Screen => data.brightness_power,
                    StatusBarItem::Heatpipe => data.heatpipe_power,
                }
            };
            if let Err(error) = PowerUpdatedEvent::new(bar).emit(app) {
                log::error!("Failed to emit PowerUpdatedEvent: {error}");
            }
            if let Err(error) = (PowerTickEvent { data }).emit(app) {
                log::error!("Failed to emit PowerTickEvent: {error}");
            }
        }
        Err(error) => {
            log::error!("Failed to get IORegistry: {error}");
            if let Some(smc) = smc {
                if let Err(error) =
                    PowerUpdatedEvent::new_with(&smc, status_bar_item, show_charging).emit(app)
                {
                    log::error!("Failed to emit PowerUpdatedEvent: {error}");
                }
            }
        }
    }
}

pub fn start_sender<R: Runtime>(
    app: &impl Manager<R>,
    mut rx: mpsc::UnboundedReceiver<SenderMessage>,
) -> async_runtime::JoinHandle<()> {
    let app = app.app_handle().clone();
    let mut smc_conn = match SMCConnection::new("AppleSMC") {
        Ok(connection) => Some(connection),
        Err(error) => {
            log::warn!("AppleSMC unavailable at startup ({error}); using IORegistry fallback");
            None
        }
    };
    let mut next_smc_retry = Instant::now() + Duration::from_secs(30);

    let mut timer = make_interval(sanitize_interval(
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
        .try_get::<bool>("preference", "statusBarShowCharging")
        .unwrap_or(true);

    async_runtime::spawn(async move {
        loop {
            select! {
                _ = timer.tick() => {
                    emit_power_sample(
                        &app,
                        &mut smc_conn,
                        &mut next_smc_retry,
                        &status_bar_item,
                        show_charging,
                    );
                }
                Some(msg) = rx.recv() => match msg {
                    SenderMessage::ImmediateSend => {
                        emit_power_sample(
                            &app,
                            &mut smc_conn,
                            &mut next_smc_retry,
                            &status_bar_item,
                            show_charging,
                        );
                    },
                    SenderMessage::ChangeInterval(interval) => {
                        timer = make_interval(sanitize_interval_ms(
                            interval.as_millis() as u64,
                        ));
                    },
                    SenderMessage::ChangeStatusBarItem(item) => {
                        status_bar_item = item;
                        emit_power_sample(
                            &app,
                            &mut smc_conn,
                            &mut next_smc_retry,
                            &status_bar_item,
                            show_charging,
                        );
                    },
                    SenderMessage::StatusBarShowCharging(show) => {
                        show_charging = show;
                        emit_power_sample(
                            &app,
                            &mut smc_conn,
                            &mut next_smc_retry,
                            &status_bar_item,
                            show_charging,
                        );
                    }
                }
            }
        }
    })
}

pub fn setup_sender_with_events<R: Runtime>(app: &impl Manager<R>) {
    let app = app.app_handle();
    let (sender_tx, rx) = mpsc::unbounded_channel();
    start_sender(app, rx);

    // send an immediate update when the main window is loaded
    let tx = sender_tx.clone();
    WindowLoadedEvent::listen(app, move |_| {
        if let Err(error) = tx.send(SenderMessage::ImmediateSend) {
            log::error!("Failed to request immediate power update: {error}");
        }
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
            if let Err(error) = tx.send(msg) {
                log::error!("Failed to apply preference update: {error}");
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_interval_is_clamped_to_safe_bounds() {
        assert_eq!(sanitize_interval_ms(1), Duration::from_millis(500));
        assert_eq!(
            sanitize_interval_ms(1_000_000),
            Duration::from_millis(60_000)
        );
        assert_eq!(sanitize_interval_ms(2_000), Duration::from_millis(2_000));
    }

    #[test]
    fn tray_prefers_delivery_power_while_charging() {
        let smc = SMCPowerData {
            charging_status: 1.0,
            delivery_rate: 31.5,
            system_total: 12.0,
            ..Default::default()
        };
        assert_eq!(status_bar_text(&smc, &StatusBarItem::System, true), 31.5);
        assert_eq!(status_bar_text(&smc, &StatusBarItem::System, false), 12.0);
    }
}
