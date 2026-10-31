use zsign_gui_core::sessions::SecretStore;

pub struct SystemSecretStore;

#[cfg(target_os = "macos")]
pub struct UserPreferences {
    suite: String,
}

#[cfg(target_os = "macos")]
impl UserPreferences {
    pub fn new() -> Self {
        Self::with_suite("com.zsign.desktop")
    }

    fn with_suite(suite: &str) -> Self {
        Self {
            suite: suite.into(),
        }
    }
}

#[cfg(target_os = "macos")]
impl SecretStore for UserPreferences {
    fn set(&self, key: &str, value: &[u8]) -> Result<(), String> {
        use core_foundation::{base::TCFType, data::CFData, string::CFString};
        use core_foundation_sys::preferences::{
            CFPreferencesAppSynchronize, CFPreferencesSetAppValue,
        };
        let key = CFString::new(key);
        let suite = CFString::new(&self.suite);
        let data = CFData::from_buffer(value);
        unsafe {
            CFPreferencesSetAppValue(
                key.as_concrete_TypeRef(),
                data.as_concrete_TypeRef().cast(),
                suite.as_concrete_TypeRef(),
            );
            if CFPreferencesAppSynchronize(suite.as_concrete_TypeRef()) == 0 {
                return Err("无法保存数据到用户偏好设置".into());
            }
        }
        Ok(())
    }

    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, String> {
        use core_foundation::{base::TCFType, data::CFData, string::CFString};
        use core_foundation_sys::{
            base::CFGetTypeID,
            data::{CFDataGetTypeID, CFDataRef},
            preferences::CFPreferencesCopyAppValue,
        };
        let key = CFString::new(key);
        let suite = CFString::new(&self.suite);
        unsafe {
            let value =
                CFPreferencesCopyAppValue(key.as_concrete_TypeRef(), suite.as_concrete_TypeRef());
            if value.is_null() {
                return Ok(None);
            }
            if CFGetTypeID(value.cast()) != CFDataGetTypeID() {
                core_foundation_sys::base::CFRelease(value.cast());
                return Err("用户偏好设置中保存的数据格式无效".into());
            }
            let data = CFData::wrap_under_create_rule(value as CFDataRef);
            Ok(Some(data.bytes().to_vec()))
        }
    }

    fn delete(&self, key: &str) -> Result<(), String> {
        use core_foundation::{base::TCFType, string::CFString};
        use core_foundation_sys::preferences::{
            CFPreferencesAppSynchronize, CFPreferencesSetAppValue,
        };
        let key = CFString::new(key);
        let suite = CFString::new(&self.suite);
        unsafe {
            CFPreferencesSetAppValue(
                key.as_concrete_TypeRef(),
                std::ptr::null(),
                suite.as_concrete_TypeRef(),
            );
            if CFPreferencesAppSynchronize(suite.as_concrete_TypeRef()) == 0 {
                return Err("无法删除用户偏好设置中的数据".into());
            }
        }
        Ok(())
    }
}

#[cfg(not(target_os = "macos"))]
pub type UserPreferences = SystemSecretStore;

#[cfg(not(target_os = "macos"))]
impl SystemSecretStore {
    pub fn new() -> Self {
        Self
    }
}

impl SystemSecretStore {
    fn entry(key: &str) -> Result<keyring::v1::Entry, String> {
        keyring::v1::Entry::new("com.zsign.desktop", key).map_err(|_| "无法访问系统安全存储".into())
    }
}

impl SecretStore for SystemSecretStore {
    fn set(&self, key: &str, value: &[u8]) -> Result<(), String> {
        Self::entry(key)?
            .set_secret(value)
            .map_err(|_| "无法保存到系统安全存储".into())
    }

    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, String> {
        match Self::entry(key)?.get_secret() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::v1::Error::NoEntry) => Ok(None),
            Err(_) => Err("无法读取系统安全存储".into()),
        }
    }

    fn delete(&self, key: &str) -> Result<(), String> {
        match Self::entry(key)?.delete_credential() {
            Ok(()) | Err(keyring::v1::Error::NoEntry) => Ok(()),
            Err(_) => Err("无法从系统安全存储删除会话".into()),
        }
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    #[test]
    fn session_preferences_persist_and_can_be_removed() {
        let suite = format!("com.zsign.desktop.test.{}", uuid::Uuid::new_v4());
        let store = UserPreferences::with_suite(&suite);
        store.set("session:test", b"test session").unwrap();
        let reopened = UserPreferences::with_suite(&suite);
        assert_eq!(
            reopened.get("session:test").unwrap(),
            Some(b"test session".to_vec())
        );
        reopened.delete("session:test").unwrap();
        assert_eq!(store.get("session:test").unwrap(), None);
    }

    #[test]
    fn signing_key_is_reused_from_user_preferences() {
        let suite = format!("com.zsign.desktop.test.{}", uuid::Uuid::new_v4());
        let store = UserPreferences::with_suite(&suite);
        let key = crate::certificates::ensure_signing_key(&store, "TEAM123456").unwrap();
        let reopened = UserPreferences::with_suite(&suite);
        let restored = crate::certificates::load_signing_key(&reopened, "TEAM123456")
            .unwrap()
            .unwrap();
        assert_eq!(key.to_public_key(), restored.to_public_key());
        reopened.delete("signing-key:TEAM123456").unwrap();
    }
}
