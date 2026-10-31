use zsign_gui_core::{Authentication, AuthenticationState};

#[test]
fn a_requested_verification_code_is_shown_before_sign_in_completes() {
    let mut auth = Authentication::new();
    assert_eq!(auth.state(), AuthenticationState::SignedOut);
    auth.start();
    auth.require_verification();
    assert_eq!(auth.state(), AuthenticationState::AwaitingVerification);
    auth.complete();
    assert_eq!(auth.state(), AuthenticationState::SignedIn);
}

#[test]
fn an_unsolicited_success_does_not_sign_in_a_signed_out_user() {
    let mut auth = Authentication::new();
    auth.complete();
    assert_eq!(auth.state(), AuthenticationState::SignedOut);
}
