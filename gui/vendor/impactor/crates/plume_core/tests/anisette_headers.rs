use std::{collections::HashMap, time::SystemTime};
use plume_core::auth::anisette_data::AnisetteData;

#[test]
fn looking_up_a_header_preserves_the_header_value_case() {
    let mut headers = HashMap::new();
    headers.insert("X-Mme-Client-Info".into(), "<Mac> <macOS;27.0;Build> <AuthKit/1>".into());
    let data = AnisetteData { base_headers: headers, generated_at: SystemTime::now(), config: Default::default() };
    assert_eq!(data.get_header("x-mme-client-info").unwrap(), "<Mac> <macOS;27.0;Build> <com.apple.AuthKit/1 (com.apple.akd/1.0)>");
}

#[test]
fn stale_anisette_refresh_failure_is_reported_instead_of_panicking() {
    let data = AnisetteData { base_headers: HashMap::new(), generated_at: SystemTime::UNIX_EPOCH, config: Default::default() };
    let account = plume_core::auth::Account::new_with_anisette(data).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    assert!(runtime.block_on(account.get_anisette()).is_err());
}
