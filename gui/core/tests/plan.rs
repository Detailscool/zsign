use zsign_gui_core::plan::{build_plan, Certificate, PlanInput};

fn input() -> PlanInput {
    PlanInput {
        app_name: "Example".into(),
        original_bundle_id: "com.example.app".into(),
        requested_bundle_id: None,
        original_registered: false,
        requested_registered: false,
        team_id: "TEAM123456".into(),
        team_name: "Test Team".into(),
        device_id: "device".into(),
        device_name: "Test iPhone".into(),
        device_registered: false,
        certificate: None,
    }
}

#[test]
fn a_new_team_gets_a_reviewable_bundle_id_and_account_changes() {
    let plan = build_plan(input()).unwrap();
    assert_eq!(
        plan.target_bundle_id,
        "com.zsign.team123456.com.example.app"
    );
    assert!(plan.register_device);
    assert!(plan.register_app_id);
    assert!(plan.create_certificate);
}

#[test]
fn existing_resources_are_reused_without_requesting_new_ones() {
    let mut data = input();
    data.original_registered = true;
    data.device_registered = true;
    data.certificate = Some(Certificate {
        serial: "serial".into(),
        name: "Development".into(),
    });
    let plan = build_plan(data).unwrap();
    assert_eq!(plan.target_bundle_id, "com.example.app");
    assert!(!plan.register_device);
    assert!(!plan.register_app_id);
    assert!(!plan.create_certificate);
}

#[test]
fn an_invalid_bundle_id_cannot_be_approved() {
    let mut data = input();
    data.requested_bundle_id = Some("../bad id".into());
    assert!(build_plan(data).is_err());
}
