use core_foundation::{
    base::{kCFAllocatorDefault, TCFType},
    data::CFData,
    dictionary::CFDictionary,
    propertylist::{CFPropertyListCreateXMLData, CFPropertyListSubClass},
};
use serde::de::DeserializeOwned;
use thiserror::Error;

#[allow(
    clippy::iter_skip_zero,
    reason = "need to return the same iterator type"
)]
pub(crate) fn skip_until<T>(
    iter: impl IntoIterator<Item = T> + ExactSizeIterator,
    width: usize,
) -> impl Iterator<Item = T> {
    let len = iter.len();
    if len < width {
        iter.into_iter().skip(0)
    } else {
        iter.into_iter().skip(len - width)
    }
}

#[derive(Debug, Error)]
pub enum DictParseError {
    #[error("Failed to create XML data")]
    XmlData,

    #[error("Failed to parse plist: {0}")]
    Deserialize(#[from] plist::Error),
}

pub fn dict_into<T: DeserializeOwned>(data: CFDictionary) -> Result<T, DictParseError> {
    let data = unsafe {
        CFPropertyListCreateXMLData(kCFAllocatorDefault, data.to_CFPropertyList().as_CFTypeRef())
    };

    if data.is_null() {
        return Err(DictParseError::XmlData);
    }

    let xml_data = unsafe { CFData::wrap_under_create_rule(data) };

    Ok(plist::from_bytes::<T>(xml_data.bytes())?)
}

pub fn get_mac_name() -> Option<String> {
    let output = std::process::Command::new("scutil")
        .arg("--get")
        .arg("ComputerName")
        .output()
        .ok()?;

    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
