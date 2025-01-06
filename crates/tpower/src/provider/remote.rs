use std::{mem, time::Duration};

use core_foundation::{base::TCFType, dictionary::CFDictionary};
use thiserror::Error;

use super::{MergedPowerData, PowerStatistic, Resource};
use crate::{
    cfdic,
    de::{repr, IORegistry},
    ffi::{smc::SMCPowerData, wrapper::ServiceConnection},
    provider::PowerDataFrom,
    util::{dict_into, DictParseError},
};

#[derive(Debug, Default)]
pub struct RemoteResource {
    pub name: String,
    pub data: IORegistry,
    pub last_update: i64,
    pub statistic: PowerStatistic,
}

impl Resource for RemoteResource {
    fn update(&mut self, data: MergedPowerData) {
        self.last_update = self.data.update_time;
        self.data = data.ioreg;
        log::info!("data.from {:?}", data.from);
        self.name = match data.from {
            PowerDataFrom::Remote((_, name, _)) => name,
            _ => String::new(),
        };

        if self.data.update_time == self.last_update {
            return;
        }

        self.statistic
            .update(self.battery_power(), self.system_in(), self.system_load());
    }

    fn system_in(&self) -> f32 {
        // self.data.
        self.data
            .power_telemetry_data
            .as_ref()
            .map_or(0.0, |d| d.system_power_in as f32)
    }

    fn system_load(&self) -> f32 {
        self.data
            .power_telemetry_data
            .as_ref()
            .map_or(0.0, |d| d.system_load as f32)
    }

    fn battery_power(&self) -> f32 {
        self.data
            .power_telemetry_data
            .as_ref()
            .map_or(0.0, |d| d.battery_power as f32)
    }

    fn adapter_power(&self) -> f32 {
        self.data.power_telemetry_data.as_ref().map_or(0.0, |d| {
            d.system_power_in as f32 + d.adapter_efficiency_loss as f32
        })
    }

    fn absolute_battery_level(&self) -> f32 {
        // self.data.battery_level as f32
        self.data.current_capacity as f32 / self.data.max_capacity as f32 * 100.0
    }

    fn is_charging(&self) -> bool {
        self.data.is_charging
    }

    fn time_remain(&self) -> Duration {
        Duration::from_secs(self.data.time_remaining as u64 * 60)
    }

    fn last_update(&self) -> Duration {
        todo!()
        // SystemTime::now()
        //     .duration_since(SystemTime::UNIX_EPOCH)
        //     .unwrap_or_default()
        //     .checked_sub(Duration::from_secs(self.last_update))
        //     .unwrap_or_default()
    }

    fn is_local(&self) -> bool {
        false
    }

    fn raw_data(&self) -> &IORegistry {
        &self.data
    }

    fn temperature(&self) -> f32 {
        self.data.temperature as f32 / 100.
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn smc(&self) -> Option<&SMCPowerData> {
        None
    }
}

#[derive(Debug, Error)]
pub enum DeviceDataError {
    #[error("Failed to send message: {0}")]
    Send(i32),
    #[error("Failed to receive message: {0}")]
    Receive(i32),
    #[error("Failed to parse message: {0}")]
    Parse(#[from] DictParseError),
}

pub fn get_device_ioreg(conn: &ServiceConnection) -> Result<IORegistry, DeviceDataError> {
    unsafe {
        conn.send(
            cfdic! {
                "EntryClass" = "IOPMPowerSource"
                "Request" = "IORegistry"
            }
            .as_concrete_TypeRef(),
        )
        .map_err(DeviceDataError::Send)
    }?;

    let response = unsafe {
        CFDictionary::wrap_under_create_rule(conn.receive().map_err(DeviceDataError::Receive)?)
    };

    let data = dict_into::<repr::IORegistryDiagnostic>(response)?;

    // SAFETY: IORegistry and repr::IORegistry are disigned to be the same
    unsafe { mem::transmute(data.diagnostics.ioregistry) }
}
