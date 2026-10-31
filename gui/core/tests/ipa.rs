use std::io::{Cursor, Write};
use zsign_gui_core::ipa::{inspect_archive, validate_entitlements};

fn archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (path, data) in entries {
        zip.start_file(*path, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(data).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

#[test]
fn an_app_extension_is_rejected_before_signing() {
    let bytes = archive(&[(
        "Payload/Example.app/PlugIns/Share.appex/Info.plist",
        b"test",
    )]);
    let error = inspect_archive(Cursor::new(bytes)).unwrap_err();
    assert!(error.contains("扩展"), "{error}");
}

const INFO: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict><key>CFBundleIdentifier</key><string>com.example.app</string><key>CFBundleExecutable</key><string>Example</string><key>CFBundleName</key><string>Example</string></dict></plist>"#;

#[test]
fn metadata_identifies_the_main_app_and_its_executable() {
    let bytes = archive(&[
        ("Payload/Example.app/Info.plist", INFO),
        ("Payload/Example.app/Example", b"binary"),
    ]);
    let info = inspect_archive(Cursor::new(bytes)).unwrap();
    assert_eq!(info.bundle_id, "com.example.app");
    assert_eq!(info.executable, "Example");
    assert_eq!(info.name, "Example");
}

#[test]
fn an_ipa_missing_its_main_executable_is_rejected() {
    let bytes = archive(&[("Payload/Example.app/Info.plist", INFO)]);
    let error = inspect_archive(Cursor::new(bytes)).unwrap_err();
    assert!(error.contains("主程序"), "{error}");
}

#[test]
fn unsafe_archive_paths_are_rejected() {
    let bytes = archive(&[("../outside", b"test")]);
    let error = inspect_archive(Cursor::new(bytes)).unwrap_err();
    assert!(error.contains("路径"), "{error}");
}

#[test]
fn nested_watch_apps_are_rejected() {
    let bytes = archive(&[(
        "Payload/Example.app/Watch/Example Watch.app/Info.plist",
        b"test",
    )]);
    let error = inspect_archive(Cursor::new(bytes)).unwrap_err();
    assert!(error.contains("嵌套"), "{error}");
}

#[test]
fn parent_segments_are_rejected_even_if_they_stay_inside_the_archive() {
    let bytes = archive(&[("Payload/Temp/../Example.app/Info.plist", INFO)]);
    let error = inspect_archive(Cursor::new(bytes)).unwrap_err();
    assert!(error.contains("路径"), "{error}");
}

#[test]
fn equivalent_archive_paths_cannot_overwrite_each_other() {
    for alias in [
        "Payload/Example.app//Example",
        "Payload/Example.app/./Example",
    ] {
        let bytes = archive(&[
            ("Payload/Example.app/Info.plist", INFO),
            ("Payload/Example.app/Example", b"binary"),
            (alias, b"different binary"),
        ]);
        let error = inspect_archive(Cursor::new(bytes)).unwrap_err();
        assert!(error.contains("路径"), "{error}");
    }
}

#[test]
fn an_app_with_a_special_capability_is_rejected() {
    let mut entitlements = plist::Dictionary::new();
    entitlements.insert(
        "application-identifier".into(),
        "TEAM123456.com.example.app".into(),
    );
    entitlements.insert(
        "com.apple.developer.team-identifier".into(),
        "TEAM123456".into(),
    );
    entitlements.insert("aps-environment".into(), "production".into());
    let error = validate_entitlements(&entitlements, "com.example.app").unwrap_err();
    assert!(error.contains("aps-environment"), "{error}");
}

#[test]
fn ordinary_application_and_its_own_keychain_group_are_supported() {
    let mut entitlements = plist::Dictionary::new();
    entitlements.insert(
        "application-identifier".into(),
        "TEAM123456.com.example.app".into(),
    );
    entitlements.insert(
        "com.apple.developer.team-identifier".into(),
        "TEAM123456".into(),
    );
    entitlements.insert(
        "keychain-access-groups".into(),
        plist::Value::Array(vec!["TEAM123456.com.example.app".into()]),
    );
    entitlements.insert("get-task-allow".into(), true.into());
    assert!(validate_entitlements(&entitlements, "com.example.app").is_ok());
}
