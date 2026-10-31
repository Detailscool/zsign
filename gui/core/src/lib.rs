//! Desktop signing workflow.

pub mod devices;
pub mod ipa;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthenticationState {
    SignedOut,
    SigningIn,
    AwaitingVerification,
    SignedIn,
}

pub struct Authentication {
    state: AuthenticationState,
}

impl Authentication {
    pub fn new() -> Self {
        Self {
            state: AuthenticationState::SignedOut,
        }
    }

    pub fn state(&self) -> AuthenticationState {
        self.state
    }

    pub fn start(&mut self) {
        self.state = AuthenticationState::SigningIn;
    }

    pub fn require_verification(&mut self) {
        self.state = AuthenticationState::AwaitingVerification;
    }

    pub fn complete(&mut self) {
        if matches!(
            self.state,
            AuthenticationState::SigningIn | AuthenticationState::AwaitingVerification
        ) {
            self.state = AuthenticationState::SignedIn;
        }
    }
}

impl Default for Authentication {
    fn default() -> Self {
        Self::new()
    }
}
