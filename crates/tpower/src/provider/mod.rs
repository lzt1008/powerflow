use std::{collections::VecDeque, time::Duration};

use anyhow::bail;
use core_foundation::{
    base::{kCFAllocatorDefault, mach_port_t, TCFType},
    dictionary::{CFDictionary, CFMutableDictionaryRef},
};
use enum_dispatch::enum_dispatch;
use io_kit_sys::{
    ret::kIOReturnSuccess, IOMasterPort, IORegistryEntryCreateCFProperties,
    IOServiceGetMatchingService, IOServiceMatching,
};
use std::{ffi::CString, mem, ops::Deref};

use crate::{
    de::{repr, IORegistry},
    ffi::{smc::SMCPowerData, InterfaceType},
    util::dict_into,
};
use ratatui::widgets::SparklineBar;

pub mod local;
pub mod remote;

pub use local::LocalResource;
pub use remote::RemoteResource;

#[enum_dispatch]
pub trait Resource {
    fn name(&self) -> String;
    /// The amount of power in (mW)
    fn system_in(&self) -> f32;

    /// The amount of system power usage (mW)
    fn system_load(&self) -> f32;

    /// The amount of battery power usage (mW)
    fn battery_power(&self) -> f32;

    /// Usually system_in + adapter_efficiency_loss
    fn adapter_power(&self) -> f32;

    fn brightness_power(&self) -> f32 {
        0.0
    }

    fn absolute_battery_level(&self) -> f32;

    fn is_charging(&self) -> bool;

    fn time_remain(&self) -> Duration;

    // Statistics
    fn max_battery_power(&self) -> f32;

    fn max_input_power(&self) -> f32;

    fn max_system_power(&self) -> f32;

    fn battery_history(&self, width: usize) -> Vec<SparklineBar>;

    fn input_history(&self, width: usize) -> Vec<SparklineBar>;

    fn system_history(&self, width: usize) -> Vec<SparklineBar>;

    fn last_update(&self) -> Duration {
        Duration::from_secs(0)
    }

    fn is_realtime(&self) -> bool;

    fn temperature(&self) -> f32;

    fn raw_data(&self) -> &IORegistry;

    fn smc(&self) -> Option<&SMCPowerData>;

    fn update(&mut self, data: MergedPowerData);
}

#[enum_dispatch(Resource)]
#[derive(Debug)]
pub enum PowerResource {
    Local(LocalResource),
    Remote(RemoteResource),
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

pub fn get_mac_ioreg() -> anyhow::Result<IORegistry> {
    let dic = get_mac_ioreg_dict()?;
    unsafe { mem::transmute(dict_into::<repr::IORegistry>(dic)) }
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
}
