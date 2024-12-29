use core::mem::size_of;
use core::str;
use io_kit_sys::types::{io_connect_t, io_service_t};
use io_kit_sys::{
    IOConnectCallStructMethod, IOIteratorNext, IOMasterPort, IOObjectRelease, IOServiceClose,
    IOServiceGetMatchingServices, IOServiceMatching, IOServiceOpen,
};
use mach::kern_return;
use mach::kern_return::kern_return_t;
use mach::port::mach_port_t;
use mach::traps::mach_task_self;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::CString;
use std::str::FromStr;

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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SMCPowerData {
    pub battery_rate: f32,
    pub delivery_rate: f32,
    pub system_total: f32,
    pub heatpipe: f32,
    pub brightness: f32,
    pub full_charge_capacity: f32,
    pub current_capacity: f32,
    pub charging_status: f32,
    pub time_to_empty: f32,
    pub time_to_full: f32,
    pub temperature: f32,
}

impl SMCPowerData {
    pub fn is_charging(&self) -> bool {
        self.charging_status > f32::EPSILON
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

fn fp_to_float32(fp: &str, bytes: &[u8; 32], _size: u32) -> Result<f32, ()> {
    let (div, signed) = match fp {
        "fp1f" => (32768.0, false),
        "fp2e" => (16384.0, false),
        "fp3d" => (8192.0, false),
        "fp4c" => (4096.0, false),
        "fp5b" => (2048.0, false),
        "fp6a" => (1024.0, false),
        "fp79" => (512.0, false),
        "fp88" => (256.0, false),
        "fpa6" => (64.0, false),
        "fpc4" => (16.0, false),
        "fpe2" => (4.0, false),
        // Signed
        "sp1e" => (16384.0, true),
        "sp2d" => (8192.0, true),
        "sp3c" => (4096.0, true),
        "sp4b" => (2048.0, true),
        "sp5a" => (1024.0, true),
        "sp69" => (512.0, true),
        "sp78" => (256.0, true),
        "sp87" => (128.0, true),
        "sp96" => (64.0, true),
        "spa5" => (32.0, true),
        "spb4" => (16.0, true),
        "spf0" => (1.0, true),
        _ => (0.0, false),
    };

    let res = u16::from_le_bytes(bytes[0..2].try_into().unwrap());

    if signed {
        Ok(res as i16 as f32 / div)
    } else {
        Ok(res as f32 / div)
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
        match SMCType::from_str(self.data_type_str()) {
            Ok(SMCType::FLT) => {
                let mut buf = [0u8; 4];
                buf.copy_from_slice(&self.bytes[0..4]);
                Some(f32::from_le_bytes(buf))
            }
            Ok(SMCType::UI8) => Some(self.bytes[0] as f32),
            Ok(SMCType::UI16) => {
                let mut buf = [0u8; 2];
                buf.copy_from_slice(&self.bytes[0..2]);
                Some(u16::from_le_bytes(buf) as f32)
            }
            Ok(SMCType::UI32) => {
                let mut buf = [0u8; 4];
                buf.copy_from_slice(&self.bytes[0..4]);
                Some(u32::from_le_bytes(buf) as f32)
            }
            Ok(SMCType::IOFT) => {
                fp_to_float32(self.data_type_str(), &self.bytes, self.data_size).ok()
            }
            Ok(_) => None,
            Err(_) => None,
        }
    }

    fn data_type_str(&self) -> &str {
        match str::from_utf8(&self.data_type) {
            Ok(s) => s,
            Err(e) => panic!("{}", e),
        }
        .trim()
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
        let key = str_to_u32(std::str::from_utf8(&val.key).unwrap());

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
