use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::{
    migrate, query, query_as,
    sqlite::{SqliteConnectOptions, SqliteQueryResult},
    SqlitePool,
};
use tauri::{
    async_runtime::{self},
    AppHandle, Manager,
};
use tokio::task::block_in_place;

use crate::history;

static DEFAULT_DATABASE_NAME: &str = "db.sqlite";

#[derive(Debug, sqlx::FromRow, Type, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChargingHistory {
    id: i64,
    from_level: i64,
    end_level: i64,
    charging_time: i64,
    timestamp: i64,
    name: String,
    udid: String,
    is_remote: i64,
    adapter_name: String,
}

pub async fn get_all_charging_history(
    conn: &SqlitePool,
) -> Result<Vec<ChargingHistory>, sqlx::Error> {
    query_as!(
        ChargingHistory,
        "SELECT id, from_level, end_level, charging_time, timestamp, name, udid, is_remote, adapter_name FROM charging_histories"
    )
    .fetch_all(conn)
    .await
}

pub async fn get_detail_by_id(conn: &SqlitePool, id: i64) -> Result<Vec<u8>, String> {
    query!("SELECT detail FROM charging_histories WHERE id = ?", id)
        .fetch_one(conn)
        .await
        .map(|v| v.detail)
        .map_err(|e| e.to_string())
}

pub async fn delete_history_by_id(
    conn: &SqlitePool,
    id: i64,
) -> Result<SqliteQueryResult, sqlx::Error> {
    query!("DELETE FROM charging_histories WHERE id = ?", id)
        .execute(conn)
        .await
}

pub async fn save_charging_history(
    conn: &SqlitePool,
    history: history::ChargingHistory,
) -> Result<SqliteQueryResult, sqlx::Error> {
    let detail = serde_json::to_vec(&history.detail).unwrap();
    let duration = history.duration;
    query!(
        "INSERT INTO charging_histories (from_level, end_level, charging_time, timestamp, detail, name, udid, is_remote, adapter_name) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        history.from_level,
        history.end_level,
        duration,
        history.timestamp,
        detail,
        history.name,
        history.udid,
        history.is_remote,
        history.adapter_name
    )
    .execute(conn)
    .await
}

pub fn setup_database(app: AppHandle) {
    block_in_place(|| {
        async_runtime::block_on(async move {
            let db_path = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data directory")
                .join(DEFAULT_DATABASE_NAME);
            let db = SqlitePool::connect_with(
                SqliteConnectOptions::new()
                    .filename(db_path)
                    .create_if_missing(true),
            )
            .await
            .unwrap();

            migrate!().run(&db).await.unwrap();

            app.manage(db);
        });
    });
}
