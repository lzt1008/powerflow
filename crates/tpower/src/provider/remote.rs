use std::mem;

use core_foundation::{base::TCFType, dictionary::CFDictionary};
use thiserror::Error;

use crate::{
    cfdic,
    de::{repr, IORegistry},
    ffi::wrapper::ServiceConnection,
    util::{dict_into, DictParseError},
};

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
