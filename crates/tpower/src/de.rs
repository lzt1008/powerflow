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
    #[repr, serde(default, rename_all(deserialize = "PascalCase", serialize = "camelCase"))]
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
        pub system_current_in: i32,
        pub system_energy_consumed: i64,
        pub system_load: i64,
        pub system_power_in: i32,
        pub system_voltage_in: i32,
    }

    // Nested `BatteryData` dictionary of `AppleSmartBattery`. Since macOS 27
    // the mAh capacity values (`AppleRawMaxCapacity`, `AppleRawCurrentCapacity`,
    // `DesignCapacity`) are only exposed here.
    #[out, serde(rename_all = "camelCase"), cfg_attr(feature = "specta", derive(specta::Type))]
    #[repr, serde(default, rename_all(deserialize = "PascalCase", serialize = "camelCase"))]
    #[derive(Debug, Clone, Default, Deserialize, Serialize)]
    pub struct BatteryData {
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
        // All scalar fields are Option<T>: IOKit's AppleSmartBattery dictionary
        // exposes different key sets across Macs (e.g. Apple silicon / M4 omits
        // `AbsoluteCapacity`, macOS 27 drops `AppleRaw*Capacity`, `DesignCapacity`
        // and `Temperature`). serde maps a missing Option field to None, so the
        // whole parse no longer fails atomically when one key is absent.
        pub absolute_capacity: Option<i32>,
        pub amperage: Option<i32>,
        pub voltage: Option<i32>,
        pub apple_raw_battery_voltage: Option<i32>,
        pub apple_raw_current_capacity: Option<i32>,
        pub apple_raw_max_capacity: Option<i32>,
        pub current_capacity: Option<i32>,
        pub cycle_count: Option<i32>,
        pub design_capacity: Option<i32>,
        pub fully_charged: Option<bool>,
        pub instant_amperage: Option<i32>,
        pub is_charging: Option<bool>,
        pub max_capacity: Option<i32>,
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
        self.battery_data_value(|b| b.full_charge_capacity)
            .or(self.apple_raw_max_capacity)
    }

    /// Remaining capacity in mAh.
    pub fn current_capacity_mah(&self) -> Option<i32> {
        self.battery_data_value(|b| b.remaining_capacity)
            .or(self.apple_raw_current_capacity)
    }

    /// Design capacity in mAh.
    pub fn design_capacity_mah(&self) -> Option<i32> {
        self.battery_data_value(|b| b.design_capacity)
            .or(self.design_capacity)
    }
}

// `provider::get_mac_ioreg` and `provider::remote` parse into the `repr::*`
// (PascalCase-deserialize) structs, then convert to their public (camelCase,
// specta-deriving) twins. These `From` impls replace an earlier `mem::transmute`
// between the twins: transmuting `repr(Rust)` structs is not a layout guarantee
// Rust makes, and the previous code even transmuted a whole `Result`, turning a
// parse error into a bad `anyhow::Error` pointer that segfaulted. A field-by-field
// move is zero-cost and fails to compile if the two ever drift apart.
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
            system_current_in: r.system_current_in,
            system_energy_consumed: r.system_energy_consumed,
            system_load: r.system_load,
            system_power_in: r.system_power_in,
            system_voltage_in: r.system_voltage_in,
        }
    }
}

impl From<repr::BatteryData> for BatteryData {
    fn from(r: repr::BatteryData) -> Self {
        Self {
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
            absolute_capacity: r.absolute_capacity,
            amperage: r.amperage,
            voltage: r.voltage,
            apple_raw_battery_voltage: r.apple_raw_battery_voltage,
            apple_raw_current_capacity: r.apple_raw_current_capacity,
            apple_raw_max_capacity: r.apple_raw_max_capacity,
            current_capacity: r.current_capacity,
            cycle_count: r.cycle_count,
            design_capacity: r.design_capacity,
            fully_charged: r.fully_charged,
            instant_amperage: r.instant_amperage,
            is_charging: r.is_charging,
            max_capacity: r.max_capacity,
            temperature: r.temperature,
            time_remaining: r.time_remaining,
            update_time: r.update_time,
        }
    }
}
