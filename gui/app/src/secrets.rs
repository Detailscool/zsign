use zsign_gui_core::sessions::SecretStore;

pub struct SystemSecretStore;

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
