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
    pub time_remain: Duration,
    pub time_remain_known: bool,
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

impl NormalizedResource {
    /// Build a local sample when AppleSMC is unavailable. IORegistry telemetry
    /// still provides the core system/battery measurements on supported Macs.
    pub fn local_from_ioreg(io: &IORegistry) -> Self {
        let mut resource = Self::from(io);
        resource.is_local = true;
        resource
    }
}

/// Resolve the real capacity values (in mAh) for battery health display.
///
/// On macOS 27+, `AppleRawMaxCapacity` / `AppleRawCurrentCapacity` /
/// `DesignCapacity` are no longer exposed at the top level of
/// `AppleSmartBattery`; the real mAh values moved into the nested
/// `BatteryData` dict (`FullChargeCapacity`, `RemainingCapacity`,
/// `DesignCapacity`). We prioritize the nested dict and fall back to the
/// top-level keys for older macOS versions.
///
/// Returns `(max_capacity, current_capacity, design_capacity)` in mAh.
fn real_capacity_from(io: &IORegistry) -> (i32, i32, i32) {
    let bd = io.battery_data.as_ref();
    (
        bd.and_then(|b| (b.full_charge_capacity > 0).then_some(b.full_charge_capacity))
            .unwrap_or(io.apple_raw_max_capacity),
        bd.and_then(|b| (b.remaining_capacity > 0).then_some(b.remaining_capacity))
            .unwrap_or(io.apple_raw_current_capacity),
        bd.and_then(|b| (b.design_capacity > 0).then_some(b.design_capacity))
            .unwrap_or(io.design_capacity),
    )
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

        let time_remain = time_remain_iokit(io.time_remaining);
        let (max_cap, cur_cap, design_cap) = real_capacity_from(io);

        Self {
            is_local: false,
            is_charging: io.is_charging,
            // Same sentinel handling as local — iOS often reports -1 / 65535
            // while TimeRemaining is still computing.
            time_remain: time_remain.unwrap_or(Duration::ZERO),
            time_remain_known: time_remain.is_some(),
            last_update: normalized_update_time(io.update_time),
            adapter_name: io
                .adapter_details
                .name
                .clone()
                .or_else(|| io.adapter_details.description.clone()),
            cycle_count: io.cycle_count,
            max_capacity: max_cap,
            design_capacity: design_cap,
            current_capacity: cur_cap,
            data: NormalizedData {
                system_in,
                system_load,
                battery_power,
                adapter_power,
                efficiency_loss,
                brightness_power: 0.,
                heatpipe_power: 0.,
                battery_level: io.current_capacity,
                absolute_battery_level: absolute_battery_level(io),
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
        let time_remain = time_remain_from(io, smc);
        let (max_cap, cur_cap, design_cap) = real_capacity_from(io);

        Self {
            is_local: true,
            last_update: normalized_update_time(io.update_time),
            // Prefer amperage / IOKit over SMC CHCC — on macOS 27 CHCC can
            // stay true while the battery is discharging.
            is_charging: is_charging_local(io, smc),
            // Prefer IORegistry's TimeRemaining (updated by IOKit every few
            // seconds) over SMC's B0TE/B0TF (which can stay stale for many
            // minutes). IOKit uses -1 (or a very large value) to signal
            // "still computing" / "unknown"; fall back to SMC in that case.
            time_remain: time_remain.unwrap_or(Duration::ZERO),
            time_remain_known: time_remain.is_some(),
            adapter_name: io
                .adapter_details
                .name
                .clone()
                .or_else(|| io.adapter_details.description.clone()),
            cycle_count: io.cycle_count,
            max_capacity: max_cap,
            design_capacity: design_cap,
            current_capacity: cur_cap,
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
                absolute_battery_level: absolute_battery_level(io),
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

const IOKIT_UNKNOWN: i32 = -1;
const IOKIT_SENTINEL: i32 = 65535;
const MAX_PLAUSIBLE_MIN: i32 = 24 * 60;

/// Convert IOKit `TimeRemaining` (minutes) to a Duration, rejecting sentinels.
fn time_remain_iokit(minutes: i32) -> Option<Duration> {
    if minutes != IOKIT_UNKNOWN
        && minutes != IOKIT_SENTINEL
        && (1..=MAX_PLAUSIBLE_MIN).contains(&minutes)
    {
        Some(Duration::from_secs((minutes as u64) * 60))
    } else {
        None
    }
}

/// Local charging detection that does not trust SMC `CHCC` alone.
fn is_charging_local(io: &IORegistry, smc: &SMCPowerData) -> bool {
    // Instant amperage sign is the most reliable signal when present.
    if io.instant_amperage != 0 {
        return io.instant_amperage > 0;
    }
    if io.amperage != 0 {
        return io.amperage > 0;
    }
    // Fall back to IOKit flag, then SMC only if IOKit is silent.
    io.is_charging || smc.is_charging()
}

/// Pick the most reliable remaining-time estimate.
///
/// IOKit exposes `TimeRemaining` (minutes) via `AppleSmartBattery` and
/// refreshes it every few seconds; SMC's `B0TE`/`B0TF` are firmware-level
/// and can stay pinned for many minutes on Apple Silicon. We therefore
/// prefer the IOKit value and only fall back to SMC when IOKit reports an
/// invalid sentinel (`-1`, `65535`, or an unreasonable magnitude).
///
/// NOTE: we intentionally do NOT use `smc.is_charging()` to pick between
/// `time_to_empty` and `time_to_full`. On macOS 27 the SMC `CHCC` key can
/// report charging=true while the battery is actually discharging
/// (amperage < 0), which would cause us to pick the wrong sentinel and
/// display "1000+ hours to full". Instead we try both SMC values and
/// keep whichever is plausible.
fn time_remain_from(io: &IORegistry, smc: &SMCPowerData) -> Option<Duration> {
    if let Some(d) = time_remain_iokit(io.time_remaining) {
        return Some(d);
    }

    // Try both SMC values; the one that's not a sentinel is the right one.
    for smc_min in [smc.time_to_empty, smc.time_to_full] {
        if smc_min.is_finite()
            && smc_min > 0.0
            && smc_min < MAX_PLAUSIBLE_MIN as f32
            && (smc_min as i32) != IOKIT_SENTINEL
        {
            return Some(Duration::from_secs_f32(60.0 * smc_min));
        }
    }

    None
}

fn normalized_update_time(value: i64) -> i64 {
    if value > 0 {
        return value;
    }
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs() as i64)
}

pub fn get_mac_ioreg_dict() -> anyhow::Result<CFDictionary> {
    let mut master_port: mach_port_t = 0;
    if unsafe { IOMasterPort(0, &mut master_port) } != 0 {
        bail!("could not get master port");
    }
    let name = CString::new("AppleSmartBattery").unwrap();
    let matching_dict = unsafe { IOServiceMatching(name.as_ptr()) };

    let service = unsafe { IOServiceGetMatchingService(master_port, matching_dict) };
    if service == 0 {
        bail!("AppleSmartBattery service not found");
    }

    let mut properties: CFMutableDictionaryRef = unsafe { mem::zeroed() };
    let kr = unsafe {
        IORegistryEntryCreateCFProperties(service, &mut properties, kCFAllocatorDefault, 0)
    };
    // IOServiceGetMatchingService returns a retained object — always release.
    unsafe { IOObjectRelease(service) };

    if kr != kIOReturnSuccess {
        bail!("could not get properties");
    }

    unsafe { Ok(CFDictionary::wrap_under_create_rule(properties)) }
}

pub fn get_mac_ioreg() -> anyhow::Result<IORegistry> {
    let dic = get_mac_ioreg_dict()?;
    Ok(dict_into::<repr::IORegistry>(dic)?.into())
}

/// Compute the absolute battery level as a percentage (0-100).
///
/// On macOS 27+ `AppleRawCurrentCapacity` / `AppleRawMaxCapacity` are no
/// longer exposed by `AppleSmartBattery`, so the raw-capacity ratio is
/// unavailable. Fall back to the `CurrentCapacity` / `MaxCapacity` pair
/// (already a 0-100 percentage) and finally to 0.0 if neither is usable.
fn absolute_battery_level(io: &IORegistry) -> f32 {
    if io.apple_raw_max_capacity > 0 {
        io.apple_raw_current_capacity as f32 / io.apple_raw_max_capacity as f32 * 100.
    } else if io.max_capacity > 0 {
        io.current_capacity as f32 / io.max_capacity as f32 * 100.
    } else {
        io.current_capacity as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_iokit_remaining_time_sentinels() {
        assert_eq!(time_remain_iokit(-1), None);
        assert_eq!(time_remain_iokit(65_535), None);
        assert_eq!(time_remain_iokit(24 * 60 + 1), None);
        assert_eq!(time_remain_iokit(90), Some(Duration::from_secs(5_400)));
    }

    #[test]
    fn amperage_overrides_stale_smc_charging_flag() {
        let io = IORegistry {
            instant_amperage: -250,
            is_charging: true,
            ..Default::default()
        };
        let smc = SMCPowerData {
            charging_status: 1.0,
            ..Default::default()
        };
        assert!(!is_charging_local(&io, &smc));
    }

    #[test]
    fn missing_update_time_uses_current_epoch() {
        let resource = NormalizedResource::from(&IORegistry::default());
        assert!(resource.last_update > 0);
        assert!(!resource.time_remain_known);
    }

    #[test]
    fn capacity_percentage_is_used_when_raw_capacity_is_missing() {
        let io = IORegistry {
            current_capacity: 75,
            max_capacity: 100,
            ..Default::default()
        };
        assert_eq!(absolute_battery_level(&io), 75.0);
    }

    #[test]
    fn real_capacity_from_nested_battery_data_on_macos_27() {
        // macOS 27: top-level AppleRaw* keys are gone, but BatteryData dict
        // carries the real mAh values.
        let io = IORegistry {
            battery_data: Some(crate::de::BatteryData {
                full_charge_capacity: 6389,
                remaining_capacity: 6389,
                design_capacity: 6249,
                ..Default::default()
            }),
            // top-level raw capacity keys are 0 (missing on macOS 27)
            ..Default::default()
        };
        let (max_cap, cur_cap, design_cap) = real_capacity_from(&io);
        assert_eq!(max_cap, 6389);
        assert_eq!(cur_cap, 6389);
        assert_eq!(design_cap, 6249);

        // Battery health = 6389 / 6249 * 100 ≈ 102.2%, capped to 100%
        let health = (max_cap as f32 / design_cap as f32 * 100.0).min(100.0);
        assert_eq!(health, 100.0);
    }

    #[test]
    fn real_capacity_falls_back_to_top_level_on_older_macos() {
        // Older macOS: BatteryData dict is absent, top-level AppleRaw* keys work.
        let io = IORegistry {
            apple_raw_max_capacity: 6400,
            apple_raw_current_capacity: 6300,
            design_capacity: 6250,
            ..Default::default()
        };
        let (max_cap, cur_cap, design_cap) = real_capacity_from(&io);
        assert_eq!(max_cap, 6400);
        assert_eq!(cur_cap, 6300);
        assert_eq!(design_cap, 6250);
    }
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
