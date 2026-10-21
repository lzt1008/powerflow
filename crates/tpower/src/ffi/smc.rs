use core::{mem::size_of, str};
use std::{collections::HashMap, ffi::CString, str::FromStr};

use io_kit_sys::{
    types::{io_connect_t, io_service_t},
    IOConnectCallStructMethod, IOIteratorNext, IOMasterPort, IOObjectRelease, IOServiceClose,
    IOServiceGetMatchingServices, IOServiceMatching, IOServiceOpen,
};
use mach::{kern_return, kern_return::kern_return_t, port::mach_port_t, traps::mach_task_self};
use serde::{Deserialize, Serialize};

// Kernel values
const KERNEL_INDEX_SMC: i32 = 2;

// SMC CMD values
const CMD_READ_BYTES: u8 = 5;
const CMD_WRITE_BYTES: u8 = 6;
const CMD_READ_KEYINFO: u8 = 9;

const SMC_SENSORS: [&str; 11] = [
    "PPBR", "PDTR", "PSTR", "PHPC", "PDBR", "B0FC", "SBAR", "CHCC", "B0TE", "B0TF", "TB0T",
];

pub trait SMCReadSensor {
    fn read_sensor(&mut self) -> SMCPowerData;
}

impl SMCReadSensor for SMCConnection {
    fn read_sensor(&mut self) -> SMCPowerData {
        SMC_SENSORS
            .into_iter()
            .fold(SMCPowerData::default(), |mut acc, key| {
                if let Ok(Some(val)) = self.read_key(key).map(|v| v.value()) {
                    let val = Some(val);
                    match key {
                        "PPBR" => acc.battery_rate = val,
                        "PDTR" => acc.delivery_rate = val,
                        // System Total Power Consumed (Delayed 1 Second)
                        "PSTR" => acc.system_total = val,
                        "PHPC" => acc.heatpipe = val,
                        "PDBR" => acc.brightness = val,
                        "B0FC" => acc.full_charge_capacity = val,
                        "SBAR" => acc.current_capacity = val,
                        "CHCC" => acc.charging_status = val,
                        "B0TE" => acc.time_to_empty = val,
                        "B0TF" => acc.time_to_full = val,
                        "TB0T" => acc.temperature = val,
                        _ => (),
                    }
                }
                acc
            })
    }
}

/// SMC readings; `None` when the key does not exist on this Mac or could not
/// be read, so callers can fall back to other sources instead of using 0.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SMCPowerData {
    pub battery_rate: Option<f32>,
    pub delivery_rate: Option<f32>,
    pub system_total: Option<f32>,
    pub heatpipe: Option<f32>,
    pub brightness: Option<f32>,
    pub full_charge_capacity: Option<f32>,
    pub current_capacity: Option<f32>,
    pub charging_status: Option<f32>,
    pub time_to_empty: Option<f32>,
    pub time_to_full: Option<f32>,
    pub temperature: Option<f32>,
}

impl SMCPowerData {
    pub fn is_charging(&self) -> bool {
        self.charging_status.is_some_and(|v| v > f32::EPSILON)
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct DataVers {
    pub major: u8,
    pub minor: u8,
    pub build: u8,
    pub reserved: [u8; 1],
    pub release: u16,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct PLimitData {
    pub version: u16,
    pub length: u16,
    pub cpu_plimit: u32,
    pub gpu_plimit: u32,
    pub mem_plimit: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct KeyInfo {
    pub data_size: u32,
    pub data_type: u32,
    pub data_attributes: u8,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct SMCKeyData {
    pub key: u32,
    pub vers: DataVers,
    pub plimit_data: PLimitData,
    pub key_info: KeyInfo,
    pub result: u8,
    pub status: u8,
    pub data8: u8,
    pub data32: u32,
    pub bytes: [u8; 32],
}

pub enum SMCType {
    CH8,
    FDS,
    FLAG,
    FLT,
    FP2E,
    FP4C,
    FP5B,
    FP88,
    FPE2,
    SI16,
    SI32,
    SI8,
    SP4B,
    SP78,
    UI16,
    UI32,
    UI8,
    IOFT,
    HEX,
}

impl FromStr for SMCType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ch8*" => Ok(SMCType::CH8),
            "{fds" => Ok(SMCType::FDS),
            "flag" => Ok(SMCType::FLAG),
            "flt" => Ok(SMCType::FLT),
            "fp2e" => Ok(SMCType::FP2E),
            "fp4c" => Ok(SMCType::FP4C),
            "fp5b" => Ok(SMCType::FP5B),
            "fp88" => Ok(SMCType::FP88),
            "fpe2" => Ok(SMCType::FPE2),
            "si16" => Ok(SMCType::SI16),
            "si32" => Ok(SMCType::SI32),
            "si8" => Ok(SMCType::SI8),
            "sp4b" => Ok(SMCType::SP4B),
            "sp78" => Ok(SMCType::SP78),
            "ui16" => Ok(SMCType::UI16),
            "ui32" => Ok(SMCType::UI32),
            "ui8" => Ok(SMCType::UI8),
            "ioft" => Ok(SMCType::IOFT),
            "_hex" => Ok(SMCType::HEX),
            _ => Err(()),
        }
    }
}

/// Decode the SMC fixed-point types (`fpXY` unsigned / `spXY` signed), where
/// `Y` is the number of fractional bits. These are always big-endian.
fn fixed_point_to_f32(data_type: &str, bytes: &[u8; 32]) -> Option<f32> {
    let mut chars = data_type.chars();
    let signed = match chars.next()? {
        'f' => false,
        's' => true,
        _ => return None,
    };
    if chars.next()? != 'p' {
        return None;
    }
    // Skip the integer-bits digit; the last hex digit is the fraction bits.
    chars.next()?.to_digit(16)?;
    let fraction_bits = chars.next()?.to_digit(16)?;
    let raw = u16::from_be_bytes([bytes[0], bytes[1]]);
    let value = if signed {
        f32::from(raw as i16)
    } else {
        f32::from(raw)
    };
    Some(value / (1u32 << fraction_bits) as f32)
}

/// Integer SMC values are little-endian on Apple silicon but big-endian on
/// Intel Macs.
fn int_from_bytes<const N: usize>(bytes: &[u8; 32]) -> u64 {
    let mut buf = [0u8; N];
    buf.copy_from_slice(&bytes[..N]);
    if cfg!(target_arch = "x86_64") {
        buf.iter().fold(0, |acc, b| (acc << 8) | u64::from(*b))
    } else {
        buf.iter()
            .rev()
            .fold(0, |acc, b| (acc << 8) | u64::from(*b))
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct SMCVal {
    pub key: [u8; 4],
    pub data_size: u32,
    pub data_type: [u8; 4],
    pub bytes: [u8; 32],
}

impl SMCVal {
    fn value(&self) -> Option<f32> {
        if self.data_size == 0 {
            // key does not exist on this machine
            return None;
        }
        let data_type = self.data_type_str()?;
        match SMCType::from_str(data_type) {
            Ok(SMCType::FLT) => {
                let mut buf = [0u8; 4];
                buf.copy_from_slice(&self.bytes[0..4]);
                Some(f32::from_ne_bytes(buf))
            }
            Ok(SMCType::UI8) => Some(f32::from(self.bytes[0])),
            Ok(SMCType::UI16) => Some(int_from_bytes::<2>(&self.bytes) as f32),
            Ok(SMCType::UI32) => Some(int_from_bytes::<4>(&self.bytes) as f32),
            Ok(SMCType::SI8) => Some(f32::from(self.bytes[0] as i8)),
            Ok(SMCType::SI16) => Some(f32::from(int_from_bytes::<2>(&self.bytes) as u16 as i16)),
            Ok(SMCType::SI32) => Some(int_from_bytes::<4>(&self.bytes) as u32 as i32 as f32),
            // 48.16 fixed point, native endianness (Apple silicon)
            Ok(SMCType::IOFT) => {
                let mut buf = [0u8; 8];
                buf.copy_from_slice(&self.bytes[0..8]);
                Some(u64::from_ne_bytes(buf) as f32 / 65536.0)
            }
            _ => fixed_point_to_f32(data_type, &self.bytes),
        }
    }

    fn data_type_str(&self) -> Option<&str> {
        str::from_utf8(&self.data_type)
            .ok()
            .map(|s| s.trim_matches(|c: char| c == '\0' || c.is_whitespace()))
    }
}

pub struct SMCConnection {
    conn: io_connect_t,
    key_info_cache: HashMap<u32, KeyInfo>,
}

impl SMCConnection {
    pub fn new(service_name: &str) -> Result<Self, kern_return_t> {
        let mut master_port: mach_port_t = 0;
        let mut iterator = 0;
        let device: io_service_t;
        let mut conn: io_connect_t = 0;

        unsafe {
            // Get master port
            let result = IOMasterPort(0, &mut master_port);
            if result != kern_return::KERN_SUCCESS {
                return Err(result);
            }

            // Create matching dictionary
            let service = CString::new(service_name).unwrap();
            let matching = IOServiceMatching(service.as_ptr());

            // Get matching services
            let result = IOServiceGetMatchingServices(master_port, matching, &mut iterator);
            if result != kern_return::KERN_SUCCESS {
                return Err(result);
            }

            // Get first device
            device = IOIteratorNext(iterator);
            if device == 0 {
                IOObjectRelease(iterator);
                return Err(kern_return::KERN_FAILURE);
            }

            // Open connection
            let result = IOServiceOpen(device, mach_task_self(), 0, &mut conn);

            // Cleanup
            IOObjectRelease(device);
            IOObjectRelease(iterator);

            if result != kern_return::KERN_SUCCESS {
                return Err(result);
            }
        }

        Ok(SMCConnection {
            conn,
            key_info_cache: HashMap::with_capacity(100),
        })
    }

    pub fn read_key(&mut self, key: &str) -> Result<SMCVal, kern_return_t> {
        let key_int = str_to_u32(key);
        let mut val = SMCVal::default();

        // First get key info from cache or SMC
        let key_info = self.get_key_info(key_int)?;

        // Setup input structure
        let input = SMCKeyData {
            key: key_int,
            data8: CMD_READ_BYTES,
            key_info,
            ..Default::default()
        };

        // Call SMC
        let output = self.call(KERNEL_INDEX_SMC, &input)?;

        // Copy data to val
        val.key.copy_from_slice(key.as_bytes());
        val.data_size = key_info.data_size;
        val.data_type
            .copy_from_slice(&u32_to_bytes(key_info.data_type));
        val.bytes = output.bytes;

        Ok(val)
    }

    fn get_key_info(&mut self, key: u32) -> Result<KeyInfo, kern_return_t> {
        // Try cache first
        if let Some(info) = self.key_info_cache.get(&key) {
            return Ok(*info);
        }

        // Not in cache, need to query SMC
        let input = SMCKeyData {
            key,
            data8: CMD_READ_KEYINFO,
            ..Default::default()
        };

        let output = self.call(KERNEL_INDEX_SMC, &input)?;

        // Cache the result
        let info = output.key_info;
        self.key_info_cache.insert(key, info);

        Ok(info)
    }

    #[allow(dead_code)]
    pub fn write_key(&mut self, val: &SMCVal) -> Result<(), kern_return_t> {
        let key = match std::str::from_utf8(&val.key) {
            Ok(key) => str_to_u32(key),
            Err(_) => return Err(kern_return::KERN_INVALID_ARGUMENT),
        };

        // Get key info first
        let key_info = self.get_key_info(key)?;

        // Verify data size matches
        if key_info.data_size != val.data_size {
            return Err(kern_return::KERN_INVALID_ARGUMENT);
        }

        let input = SMCKeyData {
            key,
            data8: CMD_WRITE_BYTES,
            bytes: val.bytes,
            key_info,
            ..Default::default()
        };

        self.call(KERNEL_INDEX_SMC, &input)?;
        Ok(())
    }

    fn call(&self, index: i32, input: &SMCKeyData) -> Result<SMCKeyData, kern_return_t> {
        let mut output = SMCKeyData::default();

        unsafe {
            let result = IOConnectCallStructMethod(
                self.conn,
                index as u32,
                input as *const _ as *const _,
                size_of::<SMCKeyData>(),
                &mut output as *mut _ as *mut _,
                &mut size_of::<SMCKeyData>(),
            );

            if result != kern_return::KERN_SUCCESS {
                return Err(result);
            }
        }

        Ok(output)
    }
}

impl Drop for SMCConnection {
    fn drop(&mut self) {
        unsafe {
            IOServiceClose(self.conn);
        }
    }
}

fn str_to_u32(s: &str) -> u32 {
    let bytes = s.as_bytes();
    ((bytes[0] as u32) << 24)
        | ((bytes[1] as u32) << 16)
        | ((bytes[2] as u32) << 8)
        | (bytes[3] as u32)
}

fn u32_to_bytes(val: u32) -> [u8; 4] {
    [
        (val >> 24) as u8,
        (val >> 16) as u8,
        (val >> 8) as u8,
        val as u8,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn val(data_type: &[u8; 4], size: u32, bytes: &[u8]) -> SMCVal {
        let mut v = SMCVal {
            data_type: *data_type,
            data_size: size,
            ..Default::default()
        };
        v.bytes[..bytes.len()].copy_from_slice(bytes);
        v
    }

    #[test]
    fn decodes_fixed_point_big_endian() {
        // sp78: 0x1a80 = 26.5
        assert_eq!(val(b"sp78", 2, &[0x1a, 0x80]).value(), Some(26.5));
        // fpe2: 0x0010 = 4.0
        assert_eq!(val(b"fpe2", 2, &[0x00, 0x10]).value(), Some(4.0));
        // sp96 signed negative: 0xff80 = -2.0
        assert_eq!(val(b"sp96", 2, &[0xff, 0x80]).value(), Some(-2.0));
    }

    #[test]
    fn decodes_float_and_missing_keys() {
        assert_eq!(val(b"flt ", 4, &1.5f32.to_ne_bytes()).value(), Some(1.5));
        assert_eq!(val(b"\0\0\0\0", 0, &[]).value(), None);
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn decodes_integers_little_endian_on_apple_silicon() {
        // B0FC on an M2 MacBook Air: 0x1189 = 4489 mAh
        assert_eq!(val(b"ui16", 2, &[0x89, 0x11]).value(), Some(4489.0));
        assert_eq!(val(b"si16", 2, &[0xc5, 0xff]).value(), Some(-59.0));
    }
}
