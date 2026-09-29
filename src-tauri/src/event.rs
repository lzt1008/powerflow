use serde::{Deserialize, Deserializer, Serialize};
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

#[derive(Serialize, Debug, Clone, Default, Type)]
#[serde(rename_all = "camelCase")]
pub enum StatusBarItem {
    #[default]
    System,
    Screen,
    Heatpipe,
}

// Unknown values (e.g. stale preferences written by other builds) fall back to
// `System` instead of failing deserialization inside an event listener.
impl<'de> Deserialize<'de> for StatusBarItem {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match String::deserialize(deserializer)?.as_str() {
            "screen" => Self::Screen,
            "heatpipe" => Self::Heatpipe,
            _ => Self::System,
        })
    }
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
    HideOnStartup(bool),
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
