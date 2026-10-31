use std::{collections::HashMap, mem, ops::Div};

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tauri::{async_runtime, AppHandle, Manager};
use tauri_specta::{Event, TypedEvent};
use tokio::sync::mpsc;
use tpower::{
    provider::{NormalizedData, NormalizedResource},
    util::get_mac_name,
};

use crate::{
    database::save_charging_history,
    device::{DevicePowerTickEvent, DeviceState},
    local::PowerTickEvent,
};

/// Keep up to 24 hours at the default 2-second interval before compacting.
const MAX_STAGED_SAMPLES: usize = 43_200;

struct ChargingHistoryStage {
    data: NormalizedResource,
    raw: String,
}

#[derive(Serialize, Deserialize, Type)]
pub struct ChargingHistory {
    pub is_remote: bool,
    pub name: String,
    pub udid: String,
    pub from_level: i32,
    pub end_level: i32,
    pub duration: i64,
    pub timestamp: i64,
    pub adapter_name: String,
    pub detail: ChargingHistoryDetail,
}

#[derive(Serialize, Deserialize, Type)]
pub struct ChargingHistoryDetail {
    avg: NormalizedData,
    peak: NormalizedData,
    curve: Vec<NormalizedResource>,
    raw: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize, Type, Event)]
pub struct HistoryRecordedEvent;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum DeviceType {
    Local,
    Remote(String),
}

fn summrize_history(
    app: &AppHandle,
    staged: Vec<ChargingHistoryStage>,
    typ: DeviceType,
) -> Option<ChargingHistory> {
    let name = match typ {
        DeviceType::Local => get_mac_name(),
        DeviceType::Remote(ref udid) => app
            .state::<DeviceState>()
            .read()
            .ok()
            .and_then(|s| s.get(udid).map(|d| d.0.clone())),
    }
    .unwrap_or_default();

    summarize_stages(staged, typ, name)
}

fn summarize_stages(
    staged: Vec<ChargingHistoryStage>,
    typ: DeviceType,
    name: String,
) -> Option<ChargingHistory> {
    let (first, last) = (staged.first()?, staged.last()?);

    let from_level = first.data.battery_level;
    let end_level = last.data.battery_level;
    let timestamp = first.data.last_update;
    // Guard against clock skew / out-of-order samples.
    let duration = (last.data.last_update - timestamp).max(0);

    let adapter_name = last
        .data
        .adapter_name
        .clone()
        .unwrap_or("Unknown".to_string());

    let avg = staged
        .iter()
        .fold(NormalizedData::default(), |acc, cur| acc + *cur.data)
        .div(staged.len() as f32);
    let peak = staged.iter().fold(NormalizedData::default(), |acc, cur| {
        acc.max_with(&cur.data)
    });
    let (curve, raw) = staged.into_iter().map(|s| (s.data, s.raw)).unzip();

    Some(ChargingHistory {
        is_remote: matches!(typ, DeviceType::Remote(_)),
        name,
        udid: match typ {
            DeviceType::Local => "local".to_string(),
            DeviceType::Remote(ref udid) => udid.clone(),
        },
        from_level,
        end_level,
        duration,
        timestamp,
        adapter_name,
        detail: ChargingHistoryDetail {
            avg,
            peak,
            curve,
            raw,
        },
    })
}

fn compact_stages(staged: &mut Vec<ChargingHistoryStage>) {
    if staged.len() < 3 {
        return;
    }

    // Preserve the true session start/end and decimate only interior samples.
    let last_index = staged.len() - 1;
    let mut compacted = Vec::with_capacity(staged.len() / 2 + 2);
    compacted.push(ChargingHistoryStage {
        data: staged[0].data.clone(),
        raw: staged[0].raw.clone(),
    });
    for (index, sample) in staged.drain(1..last_index).enumerate() {
        if index % 2 == 0 {
            compacted.push(sample);
        }
    }
    if let Some(last) = staged.pop() {
        compacted.push(last);
    }
    *staged = compacted;
}

fn push_stage(staged: &mut Vec<ChargingHistoryStage>, data: NormalizedResource) {
    if staged.len() >= MAX_STAGED_SAMPLES {
        compact_stages(staged);
        log::warn!("charging history exceeded 24 hours; compacted interior samples");
    }
    let raw = match serde_json::to_string(&data) {
        Ok(s) => s,
        Err(err) => {
            log::error!("Failed to serialize charging sample: {err}");
            String::new()
        }
    };
    staged.push(ChargingHistoryStage { raw, data });
}

fn spawn_history_recorder(
    app: AppHandle,
    mut rx: mpsc::UnboundedReceiver<(DeviceType, NormalizedResource)>,
) {
    async_runtime::spawn(async move {
        let db = app.state::<SqlitePool>();
        let mut staged: HashMap<DeviceType, Vec<ChargingHistoryStage>> = HashMap::new();

        while let Some((typ, mut data)) = rx.recv().await {
            let full_charged = data.battery_level >= 100;
            let staged = staged.entry(typ.clone()).or_default();

            if let Some(last) = staged
                .last()
                .filter(|last| data.last_update < last.data.last_update)
            {
                log::warn!(
                    "Received out-of-order charging sample: {} < {}",
                    data.last_update,
                    last.data.last_update
                );
                if data.is_charging && !full_charged {
                    continue;
                }
                // Preserve terminal transitions (full charge/unplug) even if
                // the system clock moved backwards.
                data.last_update = last.data.last_update;
            }

            let was_charging = staged
                .last()
                .map(|last| last.data.is_charging)
                .unwrap_or(false);
            let unplugged = was_charging && !data.is_charging;
            let already_staging = !staged.is_empty();
            // Do not open a new session that starts already full; do allow
            // appending the 100% sample onto an in-progress charge.
            let should_stage = data.is_charging
                && (already_staging || !full_charged)
                && staged
                    .last()
                    .map(|last| data.last_update != last.data.last_update)
                    .unwrap_or(true);

            // Stage while charging — including the 100% sample — before finalize.
            if should_stage {
                log::info!("staged: {:#?}", staged.len() + 1);
                push_stage(staged, data);
            }

            let reached_full = full_charged
                && !staged.is_empty()
                && staged
                    .last()
                    .map(|s| s.data.battery_level >= 100)
                    .unwrap_or(false);

            if unplugged || reached_full {
                let taked = mem::take(staged);
                // filter out short history
                if taked.len() <= 2 {
                    continue;
                }

                let Some(history) = summrize_history(app.app_handle(), taked, typ) else {
                    continue;
                };

                match save_charging_history(&db, &history).await {
                    Ok(res) => {
                        log::info!(
                            "history of {} saved: {}",
                            history.udid,
                            res.last_insert_rowid()
                        );
                    }
                    Err(e) => {
                        log::error!("history save failed: {:#?}", e);
                    }
                }

                HistoryRecordedEvent.emit(&app).unwrap_or_else(|err| {
                    log::error!("Failed to emit HistoryRecordedEvent: {:?}", err)
                });
            }
        }
    });
}

pub fn setup_history_recorder(app: AppHandle) {
    let (tx, rx) = mpsc::unbounded_channel();
    let tx_cloned = tx.clone();
    PowerTickEvent::listen(&app, move |TypedEvent { payload, .. }| {
        if let Err(err) = tx_cloned.send((DeviceType::Local, payload.data)) {
            log::error!("Failed to send PowerTickEvent: {err}");
        }
    });

    let tx_cloned = tx;
    DevicePowerTickEvent::listen(&app, move |TypedEvent { payload, .. }| {
        if let Err(err) = tx_cloned.send((DeviceType::Remote(payload.udid), payload.data)) {
            log::error!("Failed to send DevicePowerTickEvent: {err}");
        }
    });
    spawn_history_recorder(app.clone(), rx);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpower::de::IORegistry;

    fn sample(level: i32, timestamp: i64) -> NormalizedResource {
        NormalizedResource::local_from_ioreg(&IORegistry {
            current_capacity: level,
            max_capacity: 100,
            update_time: timestamp,
            is_charging: true,
            instant_amperage: 100,
            ..Default::default()
        })
    }

    fn stage(level: i32, timestamp: i64) -> ChargingHistoryStage {
        ChargingHistoryStage {
            raw: "{}".to_string(),
            data: sample(level, timestamp),
        }
    }

    #[test]
    fn compaction_preserves_session_boundaries() {
        let mut staged = (0..8)
            .map(|index| stage(20 + index, 100 + i64::from(index)))
            .collect::<Vec<_>>();

        compact_stages(&mut staged);

        assert_eq!(staged.first().unwrap().data.last_update, 100);
        assert_eq!(staged.last().unwrap().data.last_update, 107);
        assert!(staged
            .windows(2)
            .all(|window| window[0].data.last_update < window[1].data.last_update));
    }

    #[test]
    fn summary_includes_full_charge_and_handles_clock_rollback() {
        let staged = vec![stage(40, 200), stage(100, 190)];
        let history = summarize_stages(staged, DeviceType::Local, String::new()).unwrap();

        assert_eq!(history.from_level, 40);
        assert_eq!(history.end_level, 100);
        assert_eq!(history.duration, 0);
    }
}
