use zsign_gui_core::profile::validate_profile;

#[test]
fn profile_must_cover_the_selected_device_and_certificate() {
    let mut p = plist::Dictionary::new();
    p.insert(
        "TeamIdentifier".into(),
        plist::Value::Array(vec!["TEAM123456".into()]),
    );
    p.insert(
        "ProvisionedDevices".into(),
        plist::Value::Array(vec!["device-a".into()]),
    );
    p.insert(
        "DeveloperCertificates".into(),
        plist::Value::Array(vec![plist::Value::Data(vec![1, 2, 3])]),
    );
    p.insert(
        "ExpirationDate".into(),
        plist::Value::Date(
            std::time::SystemTime::now()
                .checked_add(std::time::Duration::from_secs(3600))
                .unwrap()
                .into(),
        ),
    );
    let mut ent = plist::Dictionary::new();
    ent.insert(
        "application-identifier".into(),
        "TEAM123456.com.example.app".into(),
    );
    p.insert("Entitlements".into(), ent.into());
    assert!(validate_profile(&p, "TEAM123456", "com.example.app", "device-a", &[1, 2, 3]).is_ok());
    assert!(validate_profile(&p, "TEAM123456", "com.example.app", "device-b", &[1, 2, 3]).is_err());
    assert!(validate_profile(&p, "TEAM123456", "com.example.app", "device-a", &[9]).is_err());
}
