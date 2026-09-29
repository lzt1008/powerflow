use std::time::Duration;

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{async_runtime, Manager, Runtime};
use tauri_plugin_pinia::ManagerExt;
use tauri_specta::Event;
use tokio::{
    select,
    sync::mpsc,
    time::{self, Instant},
};
use tpower::{
    ffi::smc::{SMCConnection, SMCReadSensor},
    provider::{get_mac_ioreg, NormalizedResource},
};

use crate::{
    event::{PowerUpdatedEvent, PreferenceEvent, StatusBarItem, WindowLoadedEvent},
    util::log_err,
};

pub enum SenderMessage {
    ImmediateSend,
    ChangeInterval(Duration),
    ChangeStatusBarItem(StatusBarItem),
    StatusBarShowCharging(bool),
}

/// Bounds for the power-tick interval. A stale or corrupt preference must not
/// stall the chart (e.g. a persisted `updateInterval` of one day).
const MIN_INTERVAL: Duration = Duration::from_millis(500);
const MAX_INTERVAL: Duration = Duration::from_secs(60);
const SOURCE_RETRY_INTERVAL: Duration = Duration::from_secs(30);

fn make_interval(period: Duration) -> time::Interval {
    let period = period.clamp(MIN_INTERVAL, MAX_INTERVAL);
    let mut timer = time::interval(period);
    timer.set_missed_tick_behavior(time::MissedTickBehavior::Delay);
    timer
}

/// The subset of a sample the menu bar title needs, kept so preference changes
/// can redraw it without taking a new sample.
#[derive(Default, Clone, Copy)]
struct StatusBarSample {
    is_charging: bool,
    system_in: f32,
    system_load: f32,
    brightness_power: f32,
    heatpipe_power: f32,
}

impl From<&NormalizedResource> for StatusBarSample {
    fn from(data: &NormalizedResource) -> Self {
        Self {
            is_charging: data.is_charging,
            system_in: data.system_in,
            system_load: data.system_load,
            brightness_power: data.brightness_power,
            heatpipe_power: data.heatpipe_power,
        }
    }
}

impl StatusBarSample {
    fn value(&self, item: &StatusBarItem, show_charging: bool) -> f32 {
        if self.is_charging && show_charging {
            return self.system_in;
        }
        match item {
            StatusBarItem::System => self.system_load,
            StatusBarItem::Screen => self.brightness_power,
            StatusBarItem::Heatpipe => self.heatpipe_power,
        }
    }
}

impl PowerUpdatedEvent {
    /// Right-align the number with figure spaces (same width as a digit) so the
    /// menu bar item keeps a constant width.
    pub fn new(value: f32) -> Self {
        Self(format!("{value:\u{2007}>4.1} w"))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Event, Type)]
#[serde(rename_all = "camelCase")]
pub struct PowerTickEvent {
    pub data: NormalizedResource,
}

/// Reads local samples. Never panics: uses whichever of IORegistry and the
/// SMC is available, and retries a failed source (no AppleSMC access, no
/// AppleSmartBattery on desktop Macs) only occasionally instead of every tick.
struct Sampler {
    smc: Option<SMCConnection>,
    next_smc_attempt: Instant,
    next_battery_attempt: Instant,
    /// Set once AppleSmartBattery has been read; after that a failed read is
    /// treated as transient and the tick is skipped, instead of emitting a
    /// sample without battery data.
    has_battery: bool,
}

impl Sampler {
    fn new() -> Self {
        Self {
            smc: None,
            next_smc_attempt: Instant::now(),
            next_battery_attempt: Instant::now(),
            has_battery: false,
        }
    }

    fn sample(&mut self) -> Option<NormalizedResource> {
        let now = Instant::now();
        if self.smc.is_none() && now >= self.next_smc_attempt {
            match SMCConnection::new("AppleSMC") {
                Ok(conn) => self.smc = Some(conn),
                Err(e) => {
                    log::warn!("AppleSMC unavailable (kern {e}), retrying later");
                    self.next_smc_attempt = now + SOURCE_RETRY_INTERVAL;
                }
            }
        }
        let smc = self.smc.as_mut().map(SMCReadSensor::read_sensor);

        let ioreg = if now >= self.next_battery_attempt {
            match get_mac_ioreg() {
                Ok(io) => {
                    self.has_battery = true;
                    Some(io)
                }
                Err(e) if self.has_battery => {
                    log::warn!("failed to read AppleSmartBattery, skipping sample: {e:#}");
                    return None;
                }
                Err(e) => {
                    log::warn!("AppleSmartBattery unavailable ({e:#}), retrying later");
                    self.next_battery_attempt = now + SOURCE_RETRY_INTERVAL;
                    None
                }
            }
        } else {
            None
        };

        (ioreg.is_some() || smc.is_some())
            .then(|| NormalizedResource::local(ioreg.as_ref(), smc.as_ref()))
    }
}

pub fn start_sender<R: Runtime>(
    app: &impl Manager<R>,
    mut rx: mpsc::UnboundedReceiver<SenderMessage>,
) -> async_runtime::JoinHandle<()> {
    let app = app.app_handle().clone();
    let mut sampler = Sampler::new();
    let mut last = StatusBarSample::default();

    let mut timer = make_interval(Duration::from_millis(
        app.pinia()
            .try_get::<u64>("preference", "updateInterval")
            .unwrap_or(2000),
    ));
    let mut status_bar_item = app
        .pinia()
        .try_get::<StatusBarItem>("preference", "statusBarItem")
        .unwrap_or_default();
    let mut show_charging = app
        .pinia()
        .try_get::<bool>("preference", "statusBarShowCharging")
        .unwrap_or(true);

    async_runtime::spawn(async move {
        loop {
            select! {
                _ = timer.tick() => {}
                Some(msg) = rx.recv() => match msg {
                    SenderMessage::ImmediateSend => {}
                    SenderMessage::ChangeInterval(interval) => {
                        timer = make_interval(interval);
                        continue;
                    }
                    SenderMessage::ChangeStatusBarItem(item) => {
                        status_bar_item = item;
                        let value = last.value(&status_bar_item, show_charging);
                        log_err(PowerUpdatedEvent::new(value).emit(&app), "emit PowerUpdatedEvent");
                        continue;
                    }
                    SenderMessage::StatusBarShowCharging(show) => {
                        show_charging = show;
                        let value = last.value(&status_bar_item, show_charging);
                        log_err(PowerUpdatedEvent::new(value).emit(&app), "emit PowerUpdatedEvent");
                        continue;
                    }
                }
            }

            // Timer tick or immediate request: take a new sample.
            let Some(data) = sampler.sample() else {
                continue;
            };
            last = StatusBarSample::from(&data);
            let value = last.value(&status_bar_item, show_charging);
            log_err(
                PowerUpdatedEvent::new(value).emit(&app),
                "emit PowerUpdatedEvent",
            );
            log_err(PowerTickEvent { data }.emit(&app), "emit PowerTickEvent");
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
        log_err(
            tx.send(SenderMessage::ImmediateSend),
            "request power update",
        );
    });

    let tx = sender_tx;
    PreferenceEvent::listen(app, move |event| {
        let msg = match event.payload {
            PreferenceEvent::UpdateInterval(interval) => Some(SenderMessage::ChangeInterval(
                Duration::from_millis(interval.into()),
            )),
            PreferenceEvent::StatusBarItem(item) => Some(SenderMessage::ChangeStatusBarItem(item)),
            PreferenceEvent::StatusBarShowCharging(show) => {
                Some(SenderMessage::StatusBarShowCharging(show))
            }
            _ => None,
        };
        if let Some(msg) = msg {
            log_err(tx.send(msg), "apply preference update");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_bar_text_has_constant_width() {
        assert_eq!(PowerUpdatedEvent::new(3.3).0, "\u{2007}3.3 w");
        assert_eq!(PowerUpdatedEvent::new(12.0).0, "12.0 w");
    }

    #[tokio::test]
    async fn interval_is_clamped() {
        assert_eq!(
            make_interval(Duration::from_millis(1)).period(),
            MIN_INTERVAL
        );
        assert_eq!(
            make_interval(Duration::from_secs(86_400)).period(),
            MAX_INTERVAL
        );
    }
}
