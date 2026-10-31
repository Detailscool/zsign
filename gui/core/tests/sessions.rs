use std::collections::HashMap;
use std::sync::Mutex;
use zsign_gui_core::sessions::{encode_session, SecretStore, SessionVault};

#[derive(Default)]
struct MemoryStore(Mutex<HashMap<String, Vec<u8>>>);
impl SecretStore for MemoryStore {
    fn set(&self, key: &str, value: &[u8]) -> Result<(), String> {
        self.0.lock().unwrap().insert(key.into(), value.into());
        Ok(())
    }
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, String> {
        Ok(self.0.lock().unwrap().get(key).cloned())
    }
    fn delete(&self, key: &str) -> Result<(), String> {
        self.0.lock().unwrap().remove(key);
        Ok(())
    }
}

#[test]
fn a_session_is_not_saved_without_opt_in() {
    let store = MemoryStore::default();
    let vault = SessionVault::new(&store);
    vault
        .save_if_requested("person@example.com", b"session token", false)
        .unwrap();
    assert!(vault.load("person@example.com").unwrap().is_none());
}

#[test]
fn an_opted_in_session_can_be_restored_and_forgotten() {
    let store = MemoryStore::default();
    let vault = SessionVault::new(&store);
    vault
        .save_if_requested("Person@example.com", b"session token", true)
        .unwrap();
    assert_eq!(
        vault.load("person@example.com").unwrap(),
        Some(b"session token".to_vec())
    );
    vault.forget("person@example.com").unwrap();
    assert!(vault.load("person@example.com").unwrap().is_none());
}

#[test]
fn serialized_session_contains_only_the_fields_needed_to_restore() {
    let mut source = plist::Dictionary::new();
    source.insert("adsid".into(), "123".into());
    source.insert("GsIdmsToken".into(), "token".into());
    source.insert("sk".into(), plist::Value::Data(vec![1; 32]));
    source.insert("c".into(), plist::Value::Data(vec![2; 16]));
    source.insert("password".into(), "do not persist".into());
    let encoded = encode_session(&source).unwrap();
    let restored: plist::Dictionary = plist::from_bytes(&encoded).unwrap();
    assert_eq!(restored.len(), 4);
    assert!(!restored.contains_key("password"));
}
