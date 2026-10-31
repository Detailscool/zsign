//! macOS AOSKit authentication data. Missing private APIs and empty values are errors.
use std::collections::HashMap;
use std::sync::OnceLock;

use objc::{msg_send, sel, sel_impl, runtime::Class};
use objc_foundation::{INSString, NSObject, NSString};
use crate::{AnisetteError, anisette_headers_provider::AnisetteHeadersProvider};

pub struct AOSKitAnisetteProvider;

impl AOSKitAnisetteProvider {
    pub fn new() -> Result<Self, AnisetteError> {
        static LOADED: OnceLock<bool> = OnceLock::new();
        let loaded = *LOADED.get_or_init(|| unsafe {
            !libc::dlopen(b"/System/Library/PrivateFrameworks/AOSKit.framework/AOSKit\0".as_ptr().cast(), libc::RTLD_LAZY | libc::RTLD_GLOBAL).is_null()
                && !libc::dlopen(b"/System/Library/PrivateFrameworks/AuthKit.framework/AuthKit\0".as_ptr().cast(), libc::RTLD_LAZY | libc::RTLD_GLOBAL).is_null()
        });
        if !loaded { return Err(AnisetteError::UnsupportedDevice); }
        let aos = Class::get("AOSUtilities").ok_or(AnisetteError::UnsupportedDevice)?;
        let ak = Class::get("AKDevice").ok_or(AnisetteError::UnsupportedDevice)?;
        let has_otp: bool = unsafe { msg_send![aos, respondsToSelector: sel!(retrieveOTPHeadersForDSID:)] };
        let has_serial: bool = unsafe { msg_send![aos, respondsToSelector: sel!(machineSerialNumber)] };
        let has_device: bool = unsafe { msg_send![ak, respondsToSelector: sel!(currentDevice)] };
        if !has_otp || !has_serial || !has_device { return Err(AnisetteError::UnsupportedDevice); }
        Ok(Self)
    }
}

unsafe fn string_value(value: *const NSString) -> Result<String, AnisetteError> {
    if value.is_null() { return Err(AnisetteError::UnsupportedDevice); }
    let string_class = Class::get("NSString").ok_or(AnisetteError::UnsupportedDevice)?;
    let is_string: bool = msg_send![value, isKindOfClass: string_class];
    if !is_string { return Err(AnisetteError::UnsupportedDevice); }
    let result = (*value).as_str().to_string();
    if result.is_empty() { return Err(AnisetteError::UnsupportedDevice); }
    Ok(result)
}

#[cfg_attr(feature = "async", async_trait::async_trait)]
impl AnisetteHeadersProvider for AOSKitAnisetteProvider {
    #[cfg_attr(not(feature = "async"), remove_async_await::remove_async_await)]
    async fn get_anisette_headers(&mut self, _: bool) -> Result<HashMap<String, String>, AnisetteError> {
        unsafe {
            let aos = Class::get("AOSUtilities").ok_or(AnisetteError::UnsupportedDevice)?;
            let ak = Class::get("AKDevice").ok_or(AnisetteError::UnsupportedDevice)?;
            let otp_headers: *const NSObject = msg_send![aos, retrieveOTPHeadersForDSID: NSString::from_str("-2")];
            let device: *const NSObject = msg_send![ak, currentDevice];
            if otp_headers.is_null() || device.is_null() { return Err(AnisetteError::UnsupportedDevice); }
            for selector in [sel!(localUserUUID), sel!(locale), sel!(serverFriendlyDescription), sel!(uniqueDeviceIdentifier)] {
                let supported: bool = msg_send![device, respondsToSelector: selector];
                if !supported { return Err(AnisetteError::UnsupportedDevice); }
            }
            let otp: *const NSString = msg_send![otp_headers, valueForKey: NSString::from_str("X-Apple-MD")];
            let machine: *const NSString = msg_send![otp_headers, valueForKey: NSString::from_str("X-Apple-MD-M")];
            let serial: *const NSString = msg_send![aos, machineSerialNumber];
            let local_user: *const NSString = msg_send![device, localUserUUID];
            let locale: *const NSObject = msg_send![device, locale];
            if locale.is_null() { return Err(AnisetteError::UnsupportedDevice); }
            let locale_id: *const NSString = msg_send![locale, localeIdentifier];
            let client: *const NSString = msg_send![device, serverFriendlyDescription];
            let device_id: *const NSString = msg_send![device, uniqueDeviceIdentifier];
            Ok(HashMap::from([
                ("X-Apple-I-MD".into(), string_value(otp)?),
                ("X-Apple-I-MD-M".into(), string_value(machine)?),
                ("X-Apple-SRL-NO".into(), string_value(serial)?),
                ("X-Apple-I-MD-LU".into(), string_value(local_user)?),
                ("X-Apple-Locale".into(), string_value(locale_id)?),
                ("X-Mme-Client-Info".into(), string_value(client)?),
                ("X-Mme-Device-Id".into(), string_value(device_id)?),
            ]))
        }
    }
}
