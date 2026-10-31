//! Desktop signing workflow.

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
        Self { state: AuthenticationState::SignedOut }
    }

    pub fn state(&self) -> AuthenticationState { self.state }

    pub fn start(&mut self) { self.state = AuthenticationState::SigningIn; }

    pub fn require_verification(&mut self) { self.state = AuthenticationState::AwaitingVerification; }

    pub fn complete(&mut self) { self.state = AuthenticationState::SignedIn; }
}

impl Default for Authentication {
    fn default() -> Self { Self::new() }
}
