use std::time::Duration;

use tauri::{async_runtime, AppHandle, Emitter};
use tauri_plugin_pinia::ManagerExt;
use tokio::{select, sync::mpsc};
use tpower::{
    ffi::smc::{SMCConnection, SMCReadSensor},
    provider::get_mac_ioreg,
};

pub enum SenderMessage {
    ImmediateSend,
    ChangeInterval(Duration),
}

pub fn start_sender(
    app: AppHandle,
    mut rx: mpsc::Receiver<SenderMessage>,
) -> async_runtime::JoinHandle<()> {
    let mut smc_conn = SMCConnection::new("AppleSMC").unwrap();
    let mut timer = tokio::time::interval(Duration::from_millis(
        app.pinia().try_get::<u64>("preferences", "update-interval").unwrap_or(2000),
    ));

    async_runtime::spawn(async move {
        let mut current = (Default::default(), Default::default());
        loop {
            select! {
                _ = timer.tick() => {
                    let data = smc_conn.read_sensor();
                    let ioreg = get_mac_ioreg().unwrap();
                    app.emit("power-updated", format!("{:.1} w", data.system_total)).unwrap();
                    current = (data, ioreg);
                    app.emit("power-data", &current).unwrap();
                }
                Some(msg) = rx.recv() => {
                    match msg {
                        SenderMessage::ImmediateSend => {
                            app.emit("power-data", &current).unwrap();
                        },
                        SenderMessage::ChangeInterval(interval) => {
                            timer = tokio::time::interval(interval);
                        }
                    }
                }
            }
        }
    })
}
