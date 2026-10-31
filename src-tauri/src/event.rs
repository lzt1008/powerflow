use std::fmt;

use serde::{de, Deserialize, Deserializer, Serialize};
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

/// Which power metric to show in the status bar.
///
/// Implements a forgiving `Deserialize`: unknown string values (e.g. stale
/// `"none"` persisted by older builds) fall back to `System` instead of
/// panicking inside tauri-specta and killing the power-tick task.
#[derive(Serialize, Debug, Clone, Default, Type)]
#[serde(rename_all = "camelCase")]
pub enum StatusBarItem {
    #[default]
    System,
    Screen,
    Heatpipe,
}

impl<'de> Deserialize<'de> for StatusBarItem {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> de::Visitor<'de> for V {
            type Value = StatusBarItem;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a status bar item string")
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<StatusBarItem, E> {
                match v {
                    "system" => Ok(StatusBarItem::System),
                    "screen" => Ok(StatusBarItem::Screen),
                    "heatpipe" => Ok(StatusBarItem::Heatpipe),
                    other => {
                        log::warn!("Unknown StatusBarItem '{other}', falling back to System");
                        Ok(StatusBarItem::System)
                    }
                }
            }
        }
        deserializer.deserialize_str(V)
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
