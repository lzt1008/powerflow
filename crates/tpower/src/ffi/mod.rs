use std::marker::{PhantomData, PhantomPinned};

use core_foundation::propertylist::CFPropertyListFormat;
use core_foundation::string::CFStringRef;
use core_foundation::{array::CFArrayRef, dictionary::CFDictionaryRef};

use libc::{c_char, c_void};

pub mod smc;
pub mod wrapper;

pub use core_foundation;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct AMDevice {
    _data: [u8; 0],
    _marker: PhantomData<(*mut u8, PhantomPinned)>,
}

pub type AMDeviceRef = *const c_void;

#[derive(Copy, Clone, Debug, Default)]
#[repr(C)]
pub struct AMDeviceNotification {
    // pub unknown0: c_uint,
    // pub unknown1: c_uint,
    // pub unknown2: c_uint,
    // pub callback: AMDeviceNotificationCallback,
    // pub cookie: c_uint,
    _data: [u8; 0],
    _marker: PhantomData<(*mut u8, PhantomPinned)>,
}

// github.com/yury/cidre
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "specta",
    derive(specta::Type, serde::Deserialize, serde::Serialize)
)]
#[non_exhaustive]
#[repr(i32)]
pub enum Action {
    /// A device has attached. The device reference belongs to the
    /// client. It must be explicitly released, or else it will leak.
    Attached = 1,
    /// A device has detached. The device object delivered will be
    /// the same as the one delivered in the Attached notification. This
    /// device reference does not need to be released.
    Detached = 2,

    /// This notification is delivered in response to
    ///
    ///   1. A call to am::DeviceNotificationUnsubscribe().
    ///   2. An error occurred on one of the underlying notification systems
    ///      (i.e. usbmuxd or mDNSResponder crashed or stopped responding).
    ///      Unsubcribing and resubscribing may recover the notification system.
    NotificationStopped = 3,

    Paired = 4,
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct AMDeviceNotificationCallbackInfo {
    pub device: AMDeviceRef,
    pub action: Action,
    pub subscription: *mut AMDeviceNotification,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct AMDServiceConnection {
    pub unknown: [u8; 16],
    pub socket: u32,
    pub unknown2: u32,
    pub secure_io_context: *mut c_void,
    pub flags: u32,
    pub device_connection_id: u32,
    pub service_name: [c_char; 128],
}

unsafe impl Send for AMDServiceConnection {}
unsafe impl Sync for AMDServiceConnection {}

pub type AMDServiceConnectionRef = *const AMDServiceConnection;

type AMDeviceNotificationCallback =
    extern "C" fn(_: *const AMDeviceNotificationCallbackInfo, _: *mut c_void);

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(
    feature = "specta",
    derive(specta::Type, serde::Deserialize, serde::Serialize)
)]
pub enum InterfaceType {
    Unknown = 0,
    USB = 1,
    WiFi = 2,
}

#[link(name = "MobileDevice", kind = "framework")]
extern "C" {
    pub fn AMDCreateDeviceList() -> CFArrayRef;
    pub fn AMDeviceNotificationSubscribe(
        callback: AMDeviceNotificationCallback,
        unknown0: i32,
        unknown1: i32,
        context: *mut c_void,
        notification: *mut AMDeviceNotification,
    );
    pub fn AMDeviceNotificationUnsubscribe(notification: *mut c_void);
    pub fn AMDeviceNotificationSubscribeWithOptions(
        callback: AMDeviceNotificationCallback,
        minimum_interface_speed: i32,
        connection_type: i32,
        context: *mut c_void,
        ref_out: *mut c_void,
        options: CFDictionaryRef,
    );
    pub fn AMDeviceCopyDeviceIdentifier(device: AMDeviceRef) -> CFStringRef;
    pub fn AMDeviceCopyValue(
        device: AMDeviceRef,
        domain: CFStringRef,
        key: CFStringRef,
    ) -> *const c_void;
    pub fn AMDeviceGetInterfaceType(device: AMDeviceRef) -> InterfaceType;
    pub fn AMDeviceConnect(device: AMDeviceRef) -> i32;
    pub fn AMDeviceDisconnect(device: AMDeviceRef) -> i32;
    pub fn AMDeviceIsPaired(device: AMDeviceRef) -> i32;
    pub fn AMDevicePair(device: AMDeviceRef) -> i32;
    pub fn AMDeviceValidatePairing(device: AMDeviceRef) -> i32;
    pub fn AMDeviceStartSession(device: AMDeviceRef) -> i32;
    pub fn AMDeviceStopSession(device: AMDeviceRef) -> i32;
    pub fn AMDeviceSecureStartService(
        device: AMDeviceRef,
        service_name: CFStringRef,
        options: CFDictionaryRef,
        service_connection: *const AMDServiceConnectionRef,
    ) -> i32;
    pub fn AMDServiceConnectionInvalidate(connection: AMDServiceConnectionRef);
    pub fn AMDServiceConnectionSendMessage(
        connection: AMDServiceConnectionRef,
        message: CFDictionaryRef,
        format: CFPropertyListFormat,
    ) -> i32;
    pub fn AMDServiceConnectionReceiveMessage(
        connection: AMDServiceConnectionRef,
        response: *const CFDictionaryRef,
        format: *const CFPropertyListFormat,
        unknown0: *const c_void,
        unknown1: *const c_void,
        unknown2: *const c_void,
    ) -> i32;
}
