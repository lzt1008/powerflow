use std::{
    collections::VecDeque,
    ffi::CString,
    mem,
    ops::{Deref, Div},
    time::Duration,
};

use anyhow::bail;
use core_foundation::{
    base::{kCFAllocatorDefault, mach_port_t, TCFType},
    dictionary::{CFDictionary, CFMutableDictionaryRef},
};
use derive_more::Add;
use io_kit_sys::{
    ret::kIOReturnSuccess, IOMasterPort, IORegistryEntryCreateCFProperties,
    IOServiceGetMatchingService, IOServiceMatching,
};
use ratatui::widgets::SparklineBar;
use serde::{Deserialize, Serialize};

use crate::{
    de::{repr, AdapterDetails, IORegistry, PowerTelemetryData},
    ffi::{smc::SMCPowerData, InterfaceType},
    util::{dict_into, skip_until},
};

pub mod remote;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct NormalizedResource {
    pub is_local: bool,
    pub is_charging: bool,
    pub time_remain: Duration,
    pub last_update: i64,
    pub adapter_name: Option<String>,
    pub cycle_count: i32,
    pub current_capacity: i32,
    pub max_capacity: i32,
    #[serde(default)]
    pub design_capacity: i32,
    #[serde(flatten)]
    pub data: NormalizedData,
}

#[derive(Debug, Clone, Copy, Default, Add, Deserialize, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct NormalizedData {
    pub system_in: f32,
    pub system_load: f32,
    pub battery_power: f32,
    pub adapter_power: f32,
    pub efficiency_loss: f32,
    /// 0 if not available
    pub brightness_power: f32,
    /// 0 if not available
    pub heatpipe_power: f32,
    pub battery_level: i32,
    pub absolute_battery_level: f32,
    pub temperature: f32,

    pub adapter_watts: f32,
    pub adapter_voltage: f32,
    pub adapter_amperage: f32,
}

impl NormalizedData {
    pub fn max_with(self, other: &Self) -> Self {
        Self {
            system_in: self.system_in.max(other.system_in),
            system_load: self.system_load.max(other.system_load),
            battery_power: self.battery_power.max(other.battery_power),
            adapter_power: self.adapter_power.max(other.adapter_power),
            efficiency_loss: self.efficiency_loss.max(other.efficiency_loss),
            battery_level: self.battery_level.max(other.battery_level),
            absolute_battery_level: self
                .absolute_battery_level
                .max(other.absolute_battery_level),
            temperature: self.temperature.max(other.temperature),
            brightness_power: self.brightness_power.max(other.brightness_power),
            heatpipe_power: self.heatpipe_power.max(other.heatpipe_power),
            adapter_watts: self.adapter_watts.max(other.adapter_watts),
            adapter_voltage: self.adapter_voltage.max(other.adapter_voltage),
            adapter_amperage: self.adapter_amperage.max(other.adapter_amperage),
        }
    }
}

impl Div<f32> for NormalizedData {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self {
            system_in: self.system_in / rhs,
            system_load: self.system_load / rhs,
            battery_power: self.battery_power / rhs,
            adapter_power: self.adapter_power / rhs,
            efficiency_loss: self.efficiency_loss / rhs,
            brightness_power: self.brightness_power / rhs,
            heatpipe_power: self.heatpipe_power / rhs,
            battery_level: self.battery_level / rhs as i32,
            absolute_battery_level: self.absolute_battery_level / rhs,
            temperature: self.temperature / rhs,
            adapter_watts: self.adapter_watts / rhs,
            adapter_voltage: self.adapter_voltage / rhs,
            adapter_amperage: self.adapter_amperage / rhs,
        }
    }
}

impl Deref for NormalizedResource {
    type Target = NormalizedData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl From<&IORegistry> for NormalizedResource {
    fn from(io: &IORegistry) -> Self {
        let (system_in, system_load, battery_power, adapter_power, efficiency_loss) =
            if let Some(d) = io.ptd() {
                (
                    d.system_power_in as f32 / 1000.,
                    d.system_load as f32 / 1000.,
                    d.battery_power as f32 / 1000.,
                    (d.system_power_in + d.adapter_efficiency_loss) as f32 / 1000.,
                    d.adapter_efficiency_loss as f32 / 1000.,
                )
            } else {
                Default::default()
            };

        Self {
            is_local: false,
            is_charging: io.is_charging,
            time_remain: Duration::from_secs(io.time_remaining as u64 * 60),
            last_update: io.update_time,
            adapter_name: io
                .adapter_details
                .name
                .clone()
                .or_else(|| io.adapter_details.description.clone()),
            cycle_count: io.cycle_count,
            max_capacity: io.apple_raw_max_capacity,
            design_capacity: io.design_capacity,
            current_capacity: io.apple_raw_current_capacity,
            data: NormalizedData {
                system_in,
                system_load,
                battery_power,
                adapter_power,
                efficiency_loss,
                brightness_power: 0.,
                heatpipe_power: 0.,
                battery_level: io.current_capacity,
                absolute_battery_level: io.absolute_battery_level(),
                temperature: io.temperature as f32 / 100.,

                adapter_watts: io.adapter_details.watts.unwrap_or_default() as f32,
                adapter_voltage: io.adapter_details.adapter_voltage.unwrap_or_default() as f32
                    / 1000.,
                adapter_amperage: io.adapter_details.current.unwrap_or_default() as f32 / 1000.,
            },
        }
    }
}

impl From<(&IORegistry, &SMCPowerData)> for NormalizedResource {
    fn from((io, smc): (&IORegistry, &SMCPowerData)) -> Self {
        Self {
            is_local: true,
            last_update: io.update_time,
            is_charging: smc.is_charging(),
            time_remain: Duration::from_secs_f32(
                60.0 * if smc.is_charging() {
                    smc.time_to_full
                } else {
                    smc.time_to_empty
                },
            ),
            adapter_name: io
                .adapter_details
                .name
                .clone()
                .or_else(|| io.adapter_details.description.clone()),
            cycle_count: io.cycle_count,
            max_capacity: io.apple_raw_max_capacity,
            design_capacity: io.design_capacity,
            current_capacity: io.apple_raw_current_capacity,
            data: NormalizedData {
                system_in: smc.delivery_rate,
                system_load: smc.system_total,
                battery_power: smc.battery_rate.max(smc.delivery_rate - smc.system_total),
                efficiency_loss: io
                    .ptd()
                    .map_or(0.0, |d| d.adapter_efficiency_loss as f32 / 1000.),
                brightness_power: smc.brightness,
                heatpipe_power: smc.heatpipe,
                battery_level: io.current_capacity,
                absolute_battery_level: io.absolute_battery_level(),
                temperature: smc.temperature,
                adapter_power: smc.delivery_rate
                    + io.ptd()
                        .map_or(0.0, |d| d.adapter_efficiency_loss as f32 / 1000.),

                adapter_watts: io.adapter_details.watts.unwrap_or_default() as f32,
                adapter_voltage: io.adapter_details.adapter_voltage.unwrap_or_default() as f32
                    / 1000.,
                adapter_amperage: io.adapter_details.current.unwrap_or_default() as f32 / 1000.,
            },
        }
    }
}

pub fn get_mac_ioreg_dict() -> anyhow::Result<CFDictionary> {
    let mut master_port: mach_port_t = 0;
    if unsafe { IOMasterPort(0, &mut master_port) } != 0 {
        bail!("could not get master port");
    }
    let name = CString::new("AppleSmartBattery").unwrap();
    let matching_dict = unsafe { IOServiceMatching(name.as_ptr()) };

    let result = unsafe { IOServiceGetMatchingService(master_port, matching_dict) };

    let mut properties: CFMutableDictionaryRef = unsafe { mem::zeroed() };
    if unsafe { IORegistryEntryCreateCFProperties(result, &mut properties, kCFAllocatorDefault, 0) }
        != kIOReturnSuccess
    {
        bail!("could not get properties");
    }

    unsafe { Ok(CFDictionary::wrap_under_create_rule(properties)) }
}

impl From<repr::AdapterDetails> for AdapterDetails {
    fn from(value: repr::AdapterDetails) -> Self {
        Self {
            adapter_voltage: value.adapter_voltage,
            is_wireless: value.is_wireless,
            watts: value.watts,
            name: value.name,
            current: value.current,
            description: value.description,
        }
    }
}

impl From<repr::PowerTelemetryData> for PowerTelemetryData {
    fn from(value: repr::PowerTelemetryData) -> Self {
        Self {
            adapter_efficiency_loss: value.adapter_efficiency_loss,
            battery_power: value.battery_power,
            system_current_in: value.system_current_in,
            system_energy_consumed: value.system_energy_consumed,
            system_load: value.system_load,
            system_power_in: value.system_power_in,
            system_voltage_in: value.system_voltage_in,
        }
    }
}

impl From<repr::IORegistry> for IORegistry {
    fn from(value: repr::IORegistry) -> Self {
        Self {
            adapter_details: value.adapter_details.into(),
            power_telemetry_data: value.power_telemetry_data.map(Into::into),
            absolute_capacity: value.absolute_capacity,
            amperage: value.amperage,
            voltage: value.voltage,
            apple_raw_battery_voltage: value.apple_raw_battery_voltage,
            apple_raw_current_capacity: value.apple_raw_current_capacity,
            apple_raw_max_capacity: value.apple_raw_max_capacity,
            current_capacity: value.current_capacity,
            cycle_count: value.cycle_count,
            design_capacity: value.design_capacity,
            fully_charged: value.fully_charged,
            instant_amperage: value.instant_amperage,
            is_charging: value.is_charging,
            max_capacity: value.max_capacity,
            temperature: value.temperature,
            time_remaining: value.time_remaining,
            update_time: value.update_time,
        }
    }
}

pub fn get_mac_ioreg() -> anyhow::Result<IORegistry> {
    let dic = get_mac_ioreg_dict()?;
    let parsed: repr::IORegistry = dict_into(dic).map_err(anyhow::Error::from)?;
    Ok(parsed.into())
}

#[derive(Debug)]
pub struct MergedPowerData {
    pub from: PowerDataFrom,
    pub smc: Option<SMCPowerData>,
    pub ioreg: IORegistry,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PowerDataFrom {
    #[default]
    Local,
    Remote((String, String, InterfaceType)),
}

impl Deref for MergedPowerData {
    type Target = IORegistry;

    fn deref(&self) -> &Self::Target {
        &self.ioreg
    }
}

#[derive(Debug, Default)]
pub struct PowerStatistic {
    pub max_battery_power: f32,
    pub max_input_power: f32,
    pub max_system_power: f32,

    pub battery_history: VecDeque<u64>,
    pub input_history: VecDeque<u64>,
    pub system_history: VecDeque<u64>,
}

impl PowerStatistic {
    pub fn update(&mut self, battery_power: f32, input_power: f32, system_power: f32) {
        if battery_power > self.max_battery_power {
            self.max_battery_power = battery_power;
        }

        if input_power > self.max_input_power {
            self.max_input_power = input_power;
        }

        if system_power > self.max_system_power {
            self.max_system_power = system_power;
        }

        self.battery_history.push_back(battery_power.abs() as u64);
        if self.battery_history.len() > 50 {
            self.battery_history.pop_front();
        }

        self.input_history.push_back(input_power.abs() as u64);
        if self.input_history.len() > 50 {
            self.input_history.pop_front();
        }

        self.system_history.push_back(system_power.abs() as u64);
        if self.system_history.len() > 200 {
            self.system_history.pop_front();
        }
    }

    pub fn battery_history(&self, width: usize) -> Vec<SparklineBar> {
        skip_until(self.battery_history.iter(), width)
            .map(|v| SparklineBar::from(*v))
            .collect()
    }

    pub fn input_history(&self, width: usize) -> Vec<SparklineBar> {
        skip_until(self.input_history.iter(), width)
            .map(|v| SparklineBar::from(*v))
            .collect()
    }

    pub fn system_history(&self, width: usize) -> Vec<SparklineBar> {
        skip_until(self.system_history.iter(), width)
            .map(|v| SparklineBar::from(*v))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repr_registry_converts_without_loss() {
        let parsed = repr::IORegistry {
            adapter_details: repr::AdapterDetails {
                adapter_voltage: Some(20_000),
                is_wireless: Some(false),
                watts: Some(96),
                name: Some("96W USB-C Power Adapter".to_string()),
                current: Some(4_800),
                description: Some("USB-C".to_string()),
            },
            power_telemetry_data: Some(repr::PowerTelemetryData {
                adapter_efficiency_loss: 1500,
                battery_power: -5_000,
                system_current_in: 3_000,
                system_energy_consumed: 123_456,
                system_load: 12_000,
                system_power_in: 15_000,
                system_voltage_in: 20_000,
            }),
            absolute_capacity: 95,
            amperage: -1_500,
            voltage: 12_000,
            apple_raw_battery_voltage: Some(12_345),
            apple_raw_current_capacity: 8_500,
            apple_raw_max_capacity: 8_900,
            current_capacity: 95,
            cycle_count: 42,
            design_capacity: 9_000,
            fully_charged: false,
            instant_amperage: -1_500,
            is_charging: true,
            max_capacity: 95,
            temperature: 3_100,
            time_remaining: 120,
            update_time: 1_719_999_999,
        };

        let converted = IORegistry::from(parsed);

        assert_eq!(converted.adapter_details.watts, Some(96));
        assert_eq!(
            converted.adapter_details.name.as_deref(),
            Some("96W USB-C Power Adapter")
        );
        assert_eq!(
            converted.power_telemetry_data.map(|d| d.system_power_in),
            Some(15_000)
        );
        assert_eq!(converted.apple_raw_current_capacity, 8_500);
        assert_eq!(converted.cycle_count, 42);
        assert!(converted.is_charging);
    }

    #[test]
    fn repr_registry_tolerates_missing_keys() {
        // macOS 27 no longer reports `AbsoluteCapacity`; parsing must not fail.
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
    <key>CurrentCapacity</key><integer>95</integer>
    <key>IsCharging</key><true/>
    <key>AdapterDetails</key><dict>
        <key>Watts</key><integer>96</integer>
    </dict>
</dict></plist>"#;
        let parsed: repr::IORegistry =
            plist::from_bytes(xml.as_bytes()).expect("lenient registry parse");
        assert_eq!(parsed.current_capacity, 95);
        assert!(parsed.is_charging);
        assert_eq!(parsed.absolute_capacity, 0);
        assert_eq!(parsed.adapter_details.watts, Some(96));
        assert!(parsed.adapter_details.name.is_none());

        let converted = IORegistry::from(parsed);
        assert_eq!(converted.current_capacity, 95);
        assert!(converted.is_charging);
    }

    #[test]
    fn absolute_battery_level_never_nan() {
        let mut registry = IORegistry {
            current_capacity: 40,
            ..Default::default()
        };
        assert_eq!(registry.absolute_battery_level(), 40.0);

        registry.apple_raw_max_capacity = 8_900;
        registry.apple_raw_current_capacity = 8_500;
        let level = registry.absolute_battery_level();
        assert!(level.is_finite());
        assert!((level - 95.5).abs() < 0.1);
    }
}
