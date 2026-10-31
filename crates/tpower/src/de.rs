use std::ops::Deref;

use serde::{Deserialize, Serialize};

macro_rules! with_repr {
    ($(
        #[out, $($out:meta),*]
        #[repr, $repr:meta]
        #[$($meta:meta),*]
        $item:item
    )*) => {
        $(
            $(#[$meta])*
            $(#[$out])*
            $item
        )*

        pub mod repr {
            use super::*;
            $(
                $(#[$meta])*
                #[$repr]
                $item
            )*
        }
    };
}

with_repr! {
    #[out, serde(rename_all = "camelCase"), cfg_attr(feature = "specta", derive(specta::Type))]
    #[repr, serde(rename_all(deserialize = "PascalCase", serialize = "camelCase"))]
    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub struct IORegistryDiagnostic {
        pub diagnostics: Diagnostics,
    }

    #[out, serde(rename_all = "camelCase"), cfg_attr(feature = "specta", derive(specta::Type))]
    #[repr, serde(rename_all(deserialize = "PascalCase", serialize = "camelCase"))]
    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub struct Diagnostics {
        #[serde(rename = "IORegistry")]
        pub ioregistry: IORegistry,
    }

    #[out, serde(rename_all = "camelCase"), cfg_attr(feature = "specta", derive(specta::Type))]
    #[repr, serde(rename_all(deserialize = "PascalCase", serialize = "camelCase"))]
    #[derive(Debug, Clone, Default, Deserialize, Serialize)]
    pub struct AdapterDetails {
        pub adapter_voltage: Option<i32>,
        pub is_wireless: Option<bool>,
        pub watts: Option<i32>,
        pub name: Option<String>,
        pub current: Option<i32>,
        pub description: Option<String>,
    }


    #[out, serde(rename_all = "camelCase"), cfg_attr(feature = "specta", derive(specta::Type))]
    #[repr, serde(default, rename_all(deserialize = "PascalCase", serialize = "camelCase"))]
    #[derive(Debug, Clone, Default, Deserialize, Serialize)]
    pub struct PowerTelemetryData {
        pub adapter_efficiency_loss: i32,
        pub battery_power: i64,
        pub system_load: i64,
        pub system_power_in: i32,
    }

    // Battery capacities may be nested or top-level, depending on the registry shape.
    #[out, serde(rename_all = "camelCase"), cfg_attr(feature = "specta", derive(specta::Type))]
    #[repr, serde(default, rename_all(deserialize = "PascalCase", serialize = "camelCase"))]
    #[derive(Debug, Clone, Default, Deserialize, Serialize)]
    pub struct BatteryData {
        pub apple_raw_current_capacity: Option<i32>,
        pub apple_raw_max_capacity: Option<i32>,
        pub full_charge_capacity: Option<i32>,
        pub remaining_capacity: Option<i32>,
        pub design_capacity: Option<i32>,
    }

    #[out, serde(rename_all = "camelCase"), cfg_attr(feature = "specta", derive(specta::Type))]
    #[repr, serde(rename_all(deserialize = "PascalCase", serialize = "camelCase"))]
    #[derive(Debug, Clone, Default, Deserialize, Serialize)]
    pub struct IORegistry {
        #[serde(default)]
        pub adapter_details: AdapterDetails,
        pub power_telemetry_data: Option<PowerTelemetryData>,
        pub battery_data: Option<BatteryData>,
        // Missing fields must not discard the rest of a valid sample.
        pub apple_raw_current_capacity: Option<i32>,
        pub apple_raw_max_capacity: Option<i32>,
        pub current_capacity: Option<i32>,
        pub cycle_count: Option<i32>,
        pub design_capacity: Option<i32>,
        pub is_charging: Option<bool>,
        pub temperature: Option<i32>,
        pub time_remaining: Option<i32>,
        pub update_time: Option<i64>,
    }
}

impl Deref for IORegistry {
    type Target = Option<PowerTelemetryData>;
    fn deref(&self) -> &Self::Target {
        &self.power_telemetry_data
    }
}

impl IORegistry {
    pub fn ptd(&self) -> Option<&PowerTelemetryData> {
        self.power_telemetry_data.as_ref()
    }

    fn battery_data_value(&self, f: impl Fn(&BatteryData) -> Option<i32>) -> Option<i32> {
        self.battery_data.as_ref().and_then(f).filter(|v| *v > 0)
    }

    /// Full charge capacity in mAh. Prefers the nested `BatteryData` dict
    /// (macOS 27+) and falls back to the legacy top-level key.
    pub fn max_capacity_mah(&self) -> Option<i32> {
        self.battery_data_value(|b| {
            b.full_charge_capacity
                .filter(|v| *v > 0)
                .or(b.apple_raw_max_capacity)
        })
        .or(self.apple_raw_max_capacity.filter(|v| *v > 0))
    }

    /// Remaining capacity in mAh.
    pub fn current_capacity_mah(&self) -> Option<i32> {
        self.battery_data
            .as_ref()
            .and_then(|b| b.remaining_capacity.or(b.apple_raw_current_capacity))
            .filter(|v| *v >= 0)
            .or(self.apple_raw_current_capacity.filter(|v| *v >= 0))
    }

    /// Design capacity in mAh.
    pub fn design_capacity_mah(&self) -> Option<i32> {
        self.battery_data_value(|b| b.design_capacity)
            .or(self.design_capacity.filter(|v| *v > 0))
    }
}

// Convert explicitly: repr(Rust) twins have no guaranteed common layout,
// and a Result must never be transmuted into a different error type.
impl From<repr::AdapterDetails> for AdapterDetails {
    fn from(r: repr::AdapterDetails) -> Self {
        Self {
            adapter_voltage: r.adapter_voltage,
            is_wireless: r.is_wireless,
            watts: r.watts,
            name: r.name,
            current: r.current,
            description: r.description,
        }
    }
}

impl From<repr::PowerTelemetryData> for PowerTelemetryData {
    fn from(r: repr::PowerTelemetryData) -> Self {
        Self {
            adapter_efficiency_loss: r.adapter_efficiency_loss,
            battery_power: r.battery_power,
            system_load: r.system_load,
            system_power_in: r.system_power_in,
        }
    }
}

impl From<repr::BatteryData> for BatteryData {
    fn from(r: repr::BatteryData) -> Self {
        Self {
            apple_raw_current_capacity: r.apple_raw_current_capacity,
            apple_raw_max_capacity: r.apple_raw_max_capacity,
            full_charge_capacity: r.full_charge_capacity,
            remaining_capacity: r.remaining_capacity,
            design_capacity: r.design_capacity,
        }
    }
}

impl From<repr::IORegistry> for IORegistry {
    fn from(r: repr::IORegistry) -> Self {
        Self {
            adapter_details: r.adapter_details.into(),
            power_telemetry_data: r.power_telemetry_data.map(Into::into),
            battery_data: r.battery_data.map(Into::into),
            apple_raw_current_capacity: r.apple_raw_current_capacity,
            apple_raw_max_capacity: r.apple_raw_max_capacity,
            current_capacity: r.current_capacity,
            cycle_count: r.cycle_count,
            design_capacity: r.design_capacity,
            is_charging: r.is_charging,
            temperature: r.temperature,
            time_remaining: r.time_remaining,
            update_time: r.update_time,
        }
    }
}
