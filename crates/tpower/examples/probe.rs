//! Print the raw power sources this Mac exposes.
//!
//! Used to verify the IORegistry and SMC decoding against the values the
//! system itself reports.

use tpower::{
    ffi::smc::{SMCConnection, SMCReadSensor},
    provider::get_mac_ioreg,
};

fn main() {
    println!("== IORegistry (AppleSmartBattery) ==");
    match get_mac_ioreg() {
        Ok(io) => {
            println!(
                "  current_capacity (percent) : {}",
                io.current_capacity.unwrap_or_default()
            );
            println!(
                "  max_capacity (percent)     : {}",
                io.max_capacity.unwrap_or_default()
            );
            println!(
                "  cycle_count                : {}",
                io.cycle_count.unwrap_or_default()
            );
            println!("  is_charging                : {:?}", io.is_charging);
            println!("  fully_charged              : {:?}", io.fully_charged);
            println!("  amperage                   : {:?}", io.amperage);
            println!("  instant_amperage           : {:?}", io.instant_amperage);
            println!("  voltage                    : {:?}", io.voltage);
            println!("  time_remaining (min)       : {:?}", io.time_remaining);
            println!("  temperature                : {:?}", io.temperature);
            println!("  max_capacity_mah           : {:?}", io.max_capacity_mah());
            println!("  current_capacity_mah       : {:?}", io.current_capacity_mah());
            println!("  design_capacity_mah        : {:?}", io.design_capacity_mah());
        }
        Err(e) => println!("  FAILED: {e:#}"),
    }

    println!("== SMC (decoded) ==");
    match SMCConnection::new("AppleSMC") {
        Ok(mut conn) => {
            let d = conn.read_sensor();
            println!("  PPBR battery_rate    = {:?}", d.battery_rate);
            println!("  PDTR delivery_rate   = {:?}", d.delivery_rate);
            println!("  PSTR system_total    = {:?}", d.system_total);
            println!("  PHPC heatpipe        = {:?}", d.heatpipe);
            println!("  PDBR brightness      = {:?}", d.brightness);
            println!("  B0FC full_charge_cap = {:?}", d.full_charge_capacity);
            println!("  SBAR current_cap     = {:?}", d.current_capacity);
            println!("  CHCC charging_status = {:?}", d.charging_status);
            println!("  B0TE time_to_empty   = {:?}", d.time_to_empty);
            println!("  B0TF time_to_full    = {:?}", d.time_to_full);
            println!("  TB0T temperature     = {:?}", d.temperature);
        }
        Err(e) => println!("  FAILED to open AppleSMC: {e}"),
    }

    println!("== SMC (raw types and bytes) ==");
    match SMCConnection::new("AppleSMC") {
        Ok(mut conn) => {
            for key in [
                "PPBR", "PDTR", "PSTR", "PHPC", "PDBR", "B0FC", "SBAR", "CHCC", "B0TE", "B0TF",
                "TB0T",
            ] {
                match conn.read_key(key) {
                    Ok(v) => {
                        let dt = String::from_utf8_lossy(&v.data_type);
                        let hex: String = v.bytes[..4].iter().map(|b| format!("{b:02x}")).collect();
                        println!("  {key}  type={dt:<6} bytes={hex}  size={}", v.data_size);
                    }
                    Err(e) => println!("  {key}  ERROR {e}"),
                }
            }
        }
        Err(e) => println!("  FAILED to open AppleSMC: {e}"),
    }
}
