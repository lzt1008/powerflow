use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::Event;
use tpower::ffi::{Action, InterfaceType};

#[derive(Serialize, Deserialize, Debug, Clone, Type)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    Light,
    Dark,
    System,
}

#[derive(Serialize, Deserialize, Debug, Clone, Type)]
#[serde(rename_all = "camelCase")]
pub enum StatusBarItem {
    System,
    Screen,
    Heatpipe,
}

#[derive(Serialize, Deserialize, Debug, Clone, Event, Type)]
#[serde(rename_all = "camelCase")]
pub enum PreferenceEvent {
    Theme(Theme),
    AnimationsEnabled(bool),
    UpdateInterval(u32),
    Language(String),
    StatusBarItem(StatusBarItem),
    StatusBarShowCharging(bool),
}

#[derive(Serialize, Deserialize, Debug, Clone, Event, Type)]
#[serde(rename_all = "camelCase")]
pub struct DeviceEvent {
    pub udid: String,
    pub name: String,
    pub interface: InterfaceType,
    pub action: Action,
}

#[derive(Serialize, Deserialize, Debug, Clone, Event, Type)]
pub struct PowerUpdatedEvent(pub String);

#[derive(Serialize, Deserialize, Debug, Clone, Event, Type)]
pub struct WindowLoadedEvent;
