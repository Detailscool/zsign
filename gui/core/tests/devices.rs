use zsign_gui_core::devices::parse_devicectl_devices;

#[test]
fn lists_only_connected_physical_ios_devices() {
    let output = r#"{"result":{"devices":[
      {"identifier":"one","properties":{"hardware":{"reality":"physical","platform":"iOS","udid":"test-udid"},"connection":{"state":"connected"},"state":{"name":"Test iPhone"}}},
      {"identifier":"two","properties":{"hardware":{"reality":"simulated","platform":"iOS","udid":"sim"},"connection":{"state":"connected"},"state":{"name":"Simulator"}}},
      {"identifier":"three","properties":{"hardware":{"reality":"physical","platform":"iOS","udid":"off"},"connection":{"state":"unavailable"},"state":{"name":"Offline"}}}
    ]}}"#;
    let devices = parse_devicectl_devices(output).unwrap();
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].name, "Test iPhone");
    assert_eq!(devices[0].udid, "test-udid");
}
