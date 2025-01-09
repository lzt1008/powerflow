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
            .unwrap()
            .get(udid)
            .map(|d| d.0.clone()),
    }
    .unwrap_or_default();

    let (first, last) = (staged.first()?, staged.last()?);

    let from_level = first.data.battery_level;
    let end_level = last.data.battery_level;
    let timestamp = first.data.last_update;
    let duration = last.data.last_update - timestamp;

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

fn spawn_history_recorder(
    app: AppHandle,
    mut rx: mpsc::Receiver<(DeviceType, NormalizedResource)>,
) {
    async_runtime::spawn(async move {
        let db = app.state::<SqlitePool>();
        let mut staged: HashMap<DeviceType, Vec<ChargingHistoryStage>> = HashMap::new();

        while let Some((typ, data)) = rx.recv().await {
            let full_charged = data.battery_level == 100;

            let staged = staged.entry(typ.clone()).or_default();

            if staged
                .last()
                .map(|last| last.data.is_charging && !data.is_charging)
                .unwrap_or(false)
                || (!staged.is_empty() && full_charged)
            {
                let taked = mem::take(staged);
                // filter out short history
                if taked.len() <= 2 {
                    continue;
                }

                let history = summrize_history(app.app_handle(), taked, typ).unwrap();

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

            if staged
                .last()
                .map(|last| data.last_update != last.data.last_update)
                .unwrap_or(true)
                && data.is_charging
                && !full_charged
            {
                log::info!("staged: {:#?}", staged.len());
                staged.push(ChargingHistoryStage {
                    raw: serde_json::to_string(&data).unwrap(),
                    data,
                });
            }
        }
    });
}

pub fn setup_history_recorder(app: AppHandle) {
    let (tx, rx) = mpsc::channel(10);
    let tx_cloned = tx.clone();
    PowerTickEvent::listen(&app, move |TypedEvent { payload, .. }| {
        let tx = tx_cloned.clone();
        async_runtime::spawn(async move {
            tx.send((DeviceType::Local, payload.data))
                .await
                .unwrap_or_else(|err| {
                    log::error!("Failed to send PowerTickEvent: {:#?}", err);
                })
        });
    });

    let tx_cloned = tx.clone();
    DevicePowerTickEvent::listen(&app, move |TypedEvent { payload, .. }| {
        let tx = tx_cloned.clone();
        async_runtime::spawn(async move {
            tx.send((DeviceType::Remote(payload.udid), payload.data))
                .await
                .unwrap_or_else(|err| {
                    log::error!("Failed to send DevicePowerTickEvent: {:#?}", err);
                })
        });
    });
    spawn_history_recorder(app.clone(), rx);
}
