use std::ptr::{null, null_mut};

use core_foundation::{
    base::TCFType,
    dictionary::CFDictionaryRef,
    propertylist::kCFPropertyListXMLFormat_v1_0,
    string::{CFString, CFStringRef},
};

use crate::{
    cfstr,
    ffi::{
        AMDServiceConnectionInvalidate, AMDServiceConnectionReceiveMessage,
        AMDServiceConnectionRef, AMDServiceConnectionSendMessage, AMDeviceConnect,
        AMDeviceCopyDeviceIdentifier, AMDeviceCopyValue, AMDeviceDisconnect,
        AMDeviceGetInterfaceType, AMDeviceIsPaired, AMDevicePair, AMDeviceRef, AMDeviceRelease,
        AMDeviceRetain, AMDeviceSecureStartService, AMDeviceStartSession, AMDeviceStopSession,
        AMDeviceValidatePairing, InterfaceType,
    },
};

pub struct ServiceConnection(pub AMDServiceConnectionRef);

unsafe impl Send for ServiceConnection {}
unsafe impl Sync for ServiceConnection {}

impl ServiceConnection {
    fn start(device: AMDeviceRef, service_name: &str) -> Result<Self, DeviceError> {
        unsafe {
            let service_name = cfstr!(service_name);
            // Out-parameter: must be a mutable place, otherwise the compiler may
            // assume it is still null after the call.
            let mut service_ptr: AMDServiceConnectionRef = null_mut();

            let result = AMDeviceSecureStartService(
                device,
                service_name.as_concrete_TypeRef(),
                null_mut(),
                &mut service_ptr,
            );

            match result {
                0 if !service_ptr.is_null() => Ok(ServiceConnection(service_ptr)),
                0 => Err(DeviceError::Service(-1)),
                err => Err(DeviceError::Service(err)),
            }
        }
    }

    /// # Safety
    /// `message` must be a valid CFDictionaryRef
    pub unsafe fn send(&self, message: CFDictionaryRef) -> Result<(), i32> {
        match unsafe {
            AMDServiceConnectionSendMessage(self.0, message, kCFPropertyListXMLFormat_v1_0)
        } {
            0 => Ok(()),
            e => Err(e),
        }
    }

    pub fn receive(&self) -> Result<CFDictionaryRef, i32> {
        unsafe {
            let mut response: CFDictionaryRef = null_mut();
            match AMDServiceConnectionReceiveMessage(
                self.0,
                &mut response,
                null(),
                null(),
                null(),
                null(),
            ) {
                0 if !response.is_null() => Ok(response),
                0 => Err(-1),
                err => Err(err),
            }
        }
    }
}

impl Drop for ServiceConnection {
    fn drop(&mut self) {
        unsafe { AMDServiceConnectionInvalidate(self.0) }
    }
}

#[derive(Debug)]
pub struct Device {
    pub device: AMDeviceRef,
    pub udid: String,
    pub interface_type: InterfaceType,
    /// Only tear down what this wrapper actually set up: the detach
    /// notification creates a second wrapper around the same `AMDeviceRef`.
    connected: bool,
    session_active: bool,
}

unsafe impl Send for Device {}
unsafe impl Sync for Device {}

#[allow(dead_code)]
#[derive(Debug, thiserror::Error)]
pub enum DeviceError {
    #[error("couldn't connect: {0}")]
    Connect(i32),

    #[error("pairing failed: {0}")]
    Pair(i32),

    #[error("pairing validation failed: {0}")]
    Validate(i32),

    #[error("session failed: {0}")]
    Session(i32),

    #[error("couldn't start service: {0}")]
    Service(i32),
}

impl Device {
    /// Wrap `device`, taking a reference of our own: the notification's
    /// reference is only guaranteed for the duration of the callback.
    ///
    /// # Safety
    /// `device` must be a valid AMDeviceRef
    pub unsafe fn new(device: AMDeviceRef) -> Self {
        let device = unsafe { AMDeviceRetain(device) };
        let id = unsafe { AMDeviceCopyDeviceIdentifier(device) };
        let udid = if id.is_null() {
            String::new()
        } else {
            unsafe { CFString::wrap_under_create_rule(id) }.to_string()
        };
        Self {
            device,
            udid,
            interface_type: unsafe { AMDeviceGetInterfaceType(device) },
            connected: false,
            session_active: false,
        }
    }

    pub fn name(&self) -> String {
        let name = unsafe {
            AMDeviceCopyValue(
                self.device,
                null(),
                cfstr!("DeviceName").as_concrete_TypeRef(),
            )
        } as CFStringRef;

        if name.is_null() {
            return String::new();
        }

        unsafe { CFString::wrap_under_create_rule(name) }.to_string()
    }

    pub fn interface_type(&mut self) -> InterfaceType {
        let interface_type = unsafe { AMDeviceGetInterfaceType(self.device) };

        self.interface_type = interface_type;

        interface_type
    }

    pub fn connect(&mut self) -> Result<(), DeviceError> {
        match unsafe { AMDeviceConnect(self.device) } {
            0 => {
                self.connected = true;
                Ok(())
            }
            err => Err(DeviceError::Connect(err)),
        }
    }

    pub fn disconnect(&mut self) {
        self.stop_session();
        if self.connected {
            unsafe { AMDeviceDisconnect(self.device) };
            self.connected = false;
        }
    }

    pub fn is_paired(&self) -> bool {
        unsafe { AMDeviceIsPaired(self.device) == 1 }
    }

    pub fn pair(&self) -> Result<(), DeviceError> {
        match unsafe { AMDevicePair(self.device) } {
            0 => Ok(()),
            err => Err(DeviceError::Pair(err)),
        }
    }

    pub fn validate_pairing(&self) -> Result<(), DeviceError> {
        match unsafe { AMDeviceValidatePairing(self.device) } {
            0 => Ok(()),
            err => Err(DeviceError::Validate(err)),
        }
    }

    pub fn start_session(&mut self) -> Result<(), DeviceError> {
        match unsafe { AMDeviceStartSession(self.device) } {
            0 => {
                self.session_active = true;
                Ok(())
            }
            err => Err(DeviceError::Session(err)),
        }
    }

    pub fn stop_session(&mut self) {
        if self.session_active {
            unsafe { AMDeviceStopSession(self.device) };
            self.session_active = false;
        }
    }

    /// Connect, pair if needed and start a session. On failure, `Drop` (or
    /// the next `disconnect`) undoes whatever was set up.
    pub fn prepare_device(&mut self) -> Result<(), DeviceError> {
        self.connect()?;
        if !self.is_paired() {
            self.pair()?;
        }
        self.validate_pairing()?;
        self.start_session()
    }

    pub fn start_service(&self, service_name: &str) -> Result<ServiceConnection, DeviceError> {
        ServiceConnection::start(self.device, service_name)
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        self.disconnect();
        unsafe { AMDeviceRelease(self.device) };
    }
}
