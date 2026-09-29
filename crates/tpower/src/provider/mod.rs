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
    ret::kIOReturnSuccess, IOMasterPort, IOObjectRelease, IORegistryEntryCreateCFProperties,
    IOServiceGetMatchingService, IOServiceMatching,
};
use ratatui::widgets::SparklineBar;
use serde::{Deserialize, Serialize};

use crate::{
    de::{repr, IORegistry},
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
    /// `None` while the OS is still estimating.
    pub time_remain: Option<Duration>,
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
            battery_level: (self.battery_level as f32 / rhs).round() as i32,
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

/// IOKit reports `-1` / `65535` in `TimeRemaining` while it is still
/// computing an estimate; anything above a day is not a useful estimate either.
const MAX_PLAUSIBLE_MINUTES: f32 = 24.0 * 60.0;

/// Convert a remaining-time value in minutes into a `Duration`, rejecting the
/// "unknown" sentinels used by IOKit and the SMC.
fn plausible_minutes(minutes: f32) -> Option<Duration> {
    (minutes.is_finite() && minutes > 0.0 && minutes <= MAX_PLAUSIBLE_MINUTES)
        .then(|| Duration::from_secs_f32(minutes * 60.0))
}

fn ioreg_time_remain(io: &IORegistry) -> Option<Duration> {
    io.time_remaining.and_then(|m| plausible_minutes(m as f32))
}

fn smc_time_remain(smc: &SMCPowerData, is_charging: bool) -> Option<Duration> {
    if is_charging {
        smc.time_to_full
    } else {
        smc.time_to_empty
    }
    .and_then(plausible_minutes)
}

/// Battery charge as a percentage of max capacity. Prefers the mAh values and
/// falls back to the `CurrentCapacity` / `MaxCapacity` percentage pair (the
/// raw mAh keys are gone from the top level on macOS 27). Returns 0.0 instead
/// of NaN / inf when nothing usable is present.
fn absolute_battery_level(io: &IORegistry) -> f32 {
    [
        io.current_capacity_mah().zip(io.max_capacity_mah()),
        io.current_capacity.zip(io.max_capacity),
    ]
    .into_iter()
    .flatten()
    .find(|(_, max)| *max > 0)
    .map_or(
        io.current_capacity.unwrap_or_default() as f32,
        |(current, max)| current as f32 / max as f32 * 100.,
    )
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

impl From<&IORegistry> for NormalizedResource {
    fn from(io: &IORegistry) -> Self {
        let is_charging = io.is_charging.unwrap_or_default();
        let (system_in, system_load, battery_power, adapter_power, efficiency_loss) =
            if let Some(d) = io.ptd() {
                // macOS: use PowerTelemetryData
                (
                    d.system_power_in as f32 / 1000.,
                    d.system_load as f32 / 1000.,
                    d.battery_power as f32 / 1000.,
                    (d.system_power_in + d.adapter_efficiency_loss) as f32 / 1000.,
                    d.adapter_efficiency_loss as f32 / 1000.,
                )
            } else {
                // iOS/iPadOS: no PowerTelemetryData, estimate from
                // InstantAmperage (mA) x Voltage (mV).
                let amperage = io.instant_amperage.or(io.amperage).unwrap_or_default();
                let voltage = io
                    .voltage
                    .or(io.apple_raw_battery_voltage)
                    .unwrap_or_default();
                let battery_power = (amperage.unsigned_abs() as f32 * voltage as f32) / 1_000_000.0;
                let (system_in, system_load) = if is_charging {
                    // adapter input - battery charging power
                    let system_in =
                        (io.adapter_details.watts.unwrap_or_default() as f32).max(battery_power);
                    (system_in, (system_in - battery_power).max(0.0))
                } else {
                    (0.0, battery_power)
                };
                (system_in, system_load, battery_power, system_in, 0.0)
            };

        Self {
            is_local: false,
            is_charging,
            time_remain: ioreg_time_remain(io),
            last_update: io.update_time.filter(|t| *t > 0).unwrap_or_else(now_secs),
            adapter_name: io
                .adapter_details
                .name
                .clone()
                .or_else(|| io.adapter_details.description.clone()),
            cycle_count: io.cycle_count.unwrap_or_default(),
            max_capacity: io.max_capacity_mah().unwrap_or_default(),
            design_capacity: io.design_capacity_mah().unwrap_or_default(),
            current_capacity: io.current_capacity_mah().unwrap_or_default(),
            data: NormalizedData {
                system_in,
                system_load,
                battery_power,
                adapter_power,
                efficiency_loss,
                brightness_power: 0.,
                heatpipe_power: 0.,
                battery_level: io.current_capacity.unwrap_or_default(),
                absolute_battery_level: absolute_battery_level(io),
                temperature: io.temperature.unwrap_or_default() as f32 / 100.,

                adapter_watts: io.adapter_details.watts.unwrap_or_default() as f32,
                adapter_voltage: io.adapter_details.adapter_voltage.unwrap_or_default() as f32
                    / 1000.,
                adapter_amperage: io.adapter_details.current.unwrap_or_default() as f32 / 1000.,
            },
        }
    }
}

impl NormalizedResource {
    /// Build a sample for this Mac from whichever sources are available: the
    /// `AppleSmartBattery` IORegistry entry (absent on desktop Macs) and the
    /// SMC (power rails, temperature).
    pub fn local(io: Option<&IORegistry>, smc: Option<&SMCPowerData>) -> Self {
        let mut resource = io.map(Self::from).unwrap_or_else(|| Self {
            last_update: now_secs(),
            ..Default::default()
        });
        resource.is_local = true;

        if let Some(smc) = smc {
            // The amperage sign is the most reliable charging signal; SMC
            // `CHCC` can stay set while the battery drains on an
            // under-powered adapter (macOS 27).
            resource.is_charging = match io.and_then(|io| io.instant_amperage.or(io.amperage)) {
                Some(amperage) if amperage != 0 => amperage > 0,
                _ => resource.is_charging || smc.is_charging(),
            };
            // IOKit's `TimeRemaining` is refreshed every few seconds, while
            // the SMC `B0TE` / `B0TF` keys can stay stale for minutes.
            resource.time_remain = io
                .and_then(ioreg_time_remain)
                .or_else(|| smc_time_remain(smc, resource.is_charging));

            // Only override IORegistry values with SMC keys this Mac has.
            let data = &mut resource.data;
            if let Some(delivery) = smc.delivery_rate {
                data.system_in = delivery;
                data.adapter_power = delivery + data.efficiency_loss;
            }
            if let Some(total) = smc.system_total {
                data.system_load = total;
            }
            let net = smc.delivery_rate.zip(smc.system_total).map(|(d, t)| d - t);
            if let Some(battery) = smc.battery_rate.into_iter().chain(net).reduce(f32::max) {
                data.battery_power = battery;
            }
            data.brightness_power = smc.brightness.unwrap_or(data.brightness_power);
            data.heatpipe_power = smc.heatpipe.unwrap_or(data.heatpipe_power);
            data.temperature = smc.temperature.unwrap_or(data.temperature);
        }
        resource
    }
}

pub fn get_mac_ioreg_dict() -> anyhow::Result<CFDictionary> {
    let mut master_port: mach_port_t = 0;
    if unsafe { IOMasterPort(0, &mut master_port) } != 0 {
        bail!("could not get master port");
    }
    let name = CString::new("AppleSmartBattery").unwrap();
    let matching_dict = unsafe { IOServiceMatching(name.as_ptr()) };

    // Consumes `matching_dict`; returns 0 on Macs without a battery.
    let service = unsafe { IOServiceGetMatchingService(master_port, matching_dict) };
    if service == 0 {
        bail!("AppleSmartBattery service not found");
    }

    let mut properties: CFMutableDictionaryRef = unsafe { mem::zeroed() };
    let status = unsafe {
        IORegistryEntryCreateCFProperties(service, &mut properties, kCFAllocatorDefault, 0)
    };
    // IOServiceGetMatchingService returns a retained object.
    unsafe { IOObjectRelease(service) };

    if status != kIOReturnSuccess || properties.is_null() {
        bail!("could not get AppleSmartBattery properties (status={status})");
    }

    unsafe { Ok(CFDictionary::wrap_under_create_rule(properties)) }
}

pub fn get_mac_ioreg() -> anyhow::Result<IORegistry> {
    let dic = get_mac_ioreg_dict()?;
    Ok(dict_into::<repr::IORegistry>(dic)?.into())
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
    use crate::de::BatteryData;

    #[test]
    fn rejects_remaining_time_sentinels() {
        assert_eq!(plausible_minutes(-1.0), None);
        assert_eq!(plausible_minutes(65_535.0), None);
        assert_eq!(plausible_minutes(24.0 * 60.0 + 1.0), None);
        assert_eq!(plausible_minutes(90.0), Some(Duration::from_secs(5_400)));
    }

    #[test]
    fn amperage_overrides_stale_smc_charging_flag() {
        let io = IORegistry {
            instant_amperage: Some(-250),
            is_charging: Some(true),
            ..Default::default()
        };
        let smc = SMCPowerData {
            charging_status: Some(1.0),
            ..Default::default()
        };
        assert!(!NormalizedResource::local(Some(&io), Some(&smc)).is_charging);
    }

    #[test]
    fn missing_smc_keys_keep_ioregistry_values() {
        let io = IORegistry {
            power_telemetry_data: Some(crate::de::PowerTelemetryData {
                system_load: 7000,
                system_power_in: 20000,
                ..Default::default()
            }),
            ..Default::default()
        };
        let smc = SMCPowerData {
            heatpipe: Some(2.0),
            ..Default::default()
        };
        let r = NormalizedResource::local(Some(&io), Some(&smc));
        assert_eq!(
            (r.system_load, r.system_in, r.heatpipe_power),
            (7.0, 20.0, 2.0)
        );
    }

    #[test]
    fn capacity_prefers_nested_battery_data() {
        // macOS 27: top-level AppleRaw* / DesignCapacity are gone.
        let io = IORegistry {
            battery_data: Some(BatteryData {
                full_charge_capacity: Some(4489),
                remaining_capacity: Some(2889),
                design_capacity: Some(5760),
                ..Default::default()
            }),
            current_capacity: Some(65),
            max_capacity: Some(100),
            ..Default::default()
        };
        let r = NormalizedResource::from(&io);
        assert_eq!(
            (r.max_capacity, r.current_capacity, r.design_capacity),
            (4489, 2889, 5760)
        );
        assert!((r.absolute_battery_level - 64.357).abs() < 0.01);
    }

    #[test]
    fn capacity_falls_back_to_legacy_keys_and_percentages() {
        let io = IORegistry {
            apple_raw_max_capacity: Some(6400),
            apple_raw_current_capacity: Some(3200),
            design_capacity: Some(6250),
            ..Default::default()
        };
        let r = NormalizedResource::from(&io);
        assert_eq!(
            (r.max_capacity, r.current_capacity, r.design_capacity),
            (6400, 3200, 6250)
        );
        assert_eq!(r.absolute_battery_level, 50.0);

        let io = IORegistry {
            current_capacity: Some(75),
            max_capacity: Some(100),
            ..Default::default()
        };
        assert_eq!(absolute_battery_level(&io), 75.0);
        assert_eq!(absolute_battery_level(&IORegistry::default()), 0.0);
    }

    #[test]
    fn missing_update_time_uses_current_time() {
        assert!(NormalizedResource::from(&IORegistry::default()).last_update > 0);
    }

    #[test]
    fn ios_power_is_estimated_without_telemetry() {
        let io = IORegistry {
            instant_amperage: Some(-500),
            voltage: Some(4000),
            is_charging: Some(false),
            ..Default::default()
        };
        let r = NormalizedResource::from(&io);
        assert_eq!(r.battery_power, 2.0);
        assert_eq!(r.system_load, 2.0);
    }
}
