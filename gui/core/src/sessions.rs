use sha2::{Digest, Sha256};

/// Extract only the tokens required by Apple's app-token exchange.
pub fn encode_session(source: &plist::Dictionary) -> Result<Vec<u8>, String> {
    if source
        .get("adsid")
        .and_then(plist::Value::as_string)
        .filter(|s| !s.is_empty())
        .is_none()
        || source
            .get("GsIdmsToken")
            .and_then(plist::Value::as_string)
            .filter(|s| !s.is_empty())
            .is_none()
        || source
            .get("sk")
            .and_then(plist::Value::as_data)
            .is_none_or(|s| s.len() != 32)
        || source
            .get("c")
            .and_then(plist::Value::as_data)
            .is_none_or(|s| s.is_empty())
    {
        return Err("登录会话不完整，无法保存".into());
    }
    let mut selected = plist::Dictionary::new();
    for key in ["adsid", "GsIdmsToken", "sk", "c"] {
        selected.insert(key.into(), source[key].clone());
    }
    let mut bytes = Vec::new();
    plist::to_writer_binary(&mut bytes, &selected).map_err(|_| "无法编码登录会话")?;
    Ok(bytes)
}

pub fn decode_session(bytes: &[u8]) -> Result<plist::Dictionary, String> {
    let value: plist::Dictionary = plist::from_bytes(bytes).map_err(|_| "已保存的会话无效")?;
    let sanitized = encode_session(&value)?;
    plist::from_bytes(&sanitized).map_err(|_| "已保存的会话无效".into())
}

/// A platform-backed secret store. Implementations must never fall back to plaintext files.
pub trait SecretStore {
    fn set(&self, key: &str, value: &[u8]) -> Result<(), String>;
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, String>;
    fn delete(&self, key: &str) -> Result<(), String>;
}

pub struct SessionVault<'a, S: SecretStore> {
    store: &'a S,
}

impl<'a, S: SecretStore> SessionVault<'a, S> {
    pub fn new(store: &'a S) -> Self {
        Self { store }
    }

    fn key(email: &str) -> Result<String, String> {
        let normalized = email.trim().to_lowercase();
        if normalized.is_empty() {
            return Err("请输入 Apple ID".into());
        }
        Ok(format!(
            "session:{:x}",
            Sha256::digest(normalized.as_bytes())
        ))
    }

    /// Only session data belongs here; passwords and verification codes do not.
    pub fn save_if_requested(
        &self,
        email: &str,
        session: &[u8],
        remember: bool,
    ) -> Result<(), String> {
        if remember {
            self.store.set(&Self::key(email)?, session)?;
        }
        Ok(())
    }

    pub fn load(&self, email: &str) -> Result<Option<Vec<u8>>, String> {
        self.store.get(&Self::key(email)?)
    }

    pub fn forget(&self, email: &str) -> Result<(), String> {
        self.store.delete(&Self::key(email)?)
    }
}
