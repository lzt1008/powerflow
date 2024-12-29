use std::time::Duration;

use ratatui::widgets::SparklineBar;

use crate::{
    de::IORegistry,
    ffi::smc::SMCPowerData,
    util::{get_mac_model, skip_until},
};

use super::{MergedPowerData, PowerStatistic, Resource};

#[derive(Debug, Default)]
pub struct LocalResource {
    pub name: String,
    pub data: SMCPowerData,
    pub ioreg: IORegistry,
    pub efficiency_loss: f32,
    pub last_update: u64,
    pub statistic: PowerStatistic,
}

impl Resource for LocalResource {
    fn update(&mut self, mut data: MergedPowerData) {
        self.data = data
            .smc
            .take()
            .expect("Local Power Resource must have SMC data");

        if data.ioreg.update_time as u64 > self.last_update {
            self.last_update = data.ioreg.update_time;
            return;
        }

        self.ioreg = data.ioreg;

        self.statistic
            .update(self.battery_power(), self.system_in(), self.system_load());
    }

    fn raw_data(&self) -> &IORegistry {
        &self.ioreg
    }

    fn system_in(&self) -> f32 {
        self.data.delivery_rate * 1000.0
    }

    fn system_load(&self) -> f32 {
        self.data.system_total * 1000.0
    }

    fn battery_power(&self) -> f32 {
        self.data.battery_rate * 1000.0
    }

    fn adapter_power(&self) -> f32 {
        self.system_in() + self.efficiency_loss
    }

    fn brightness_power(&self) -> f32 {
        self.data.brightness
    }

    fn absolute_battery_level(&self) -> f32 {
        self.data.current_capacity / self.data.full_charge_capacity * 100.0
    }

    fn time_remain(&self) -> Duration {
        Duration::from_secs_f32(
            60.0 * if self.data.is_charging() {
                self.data.time_to_full
            } else {
                self.data.time_to_empty
            },
        )
    }

    fn is_charging(&self) -> bool {
        self.data.is_charging()
    }

    fn max_battery_power(&self) -> f32 {
        self.statistic.max_battery_power
    }

    fn max_input_power(&self) -> f32 {
        self.statistic.max_input_power
    }

    fn max_system_power(&self) -> f32 {
        self.statistic.max_system_power
    }

    fn battery_history(&self, width: usize) -> Vec<SparklineBar> {
        skip_until(self.statistic.battery_history.iter(), width)
            .map(|v| SparklineBar::from(*v))
            .collect()
    }

    fn input_history(&self, width: usize) -> Vec<SparklineBar> {
        skip_until(self.statistic.input_history.iter(), width)
            .map(|v| SparklineBar::from(*v))
            .collect()
    }

    fn system_history(&self, width: usize) -> Vec<SparklineBar> {
        skip_until(self.statistic.system_history.iter(), width)
            .map(|v| SparklineBar::from(*v))
            .collect()
    }

    fn is_realtime(&self) -> bool {
        true
    }

    fn temperature(&self) -> f32 {
        self.data.temperature as f32
    }

    fn name(&self) -> String {
        get_mac_model().unwrap()
    }

    fn smc(&self) -> Option<&SMCPowerData> {
        Some(&self.data)
    }
}
