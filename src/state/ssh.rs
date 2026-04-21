use serde::{Deserialize, Serialize};

/// Categorized SSH error kinds for better UX and diagnostics
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum SshErrorKind {
    /// Authentication failed (wrong password, key rejection, etc.)
    AuthenticationFailed(String),
    /// Connection failed (host unreachable, timeout, DNS, etc.)
    ConnectionFailed(String),
    /// Known host key mismatch — possible security issue
    HostKeyMismatch(String),
    /// SSH Agent unavailable or not accessible
    AgentUnavailable(String),
    /// New host key not in known_hosts — user must verify manually
    HostKeyUnknown(String),
    /// Generic or unclassified error
    Other(String),
}

impl SshErrorKind {
    /// Get a user-friendly message for this error
    pub fn message(&self) -> String {
        match self {
            SshErrorKind::AuthenticationFailed(msg) => format!("Auth failed: {}", msg),
            SshErrorKind::ConnectionFailed(msg) => format!("Connection error: {}", msg),
            SshErrorKind::HostKeyMismatch(msg) => format!("Host key mismatch: {}", msg),
            SshErrorKind::AgentUnavailable(msg) => format!("Agent unavailable: {}", msg),
            SshErrorKind::HostKeyUnknown(msg) => format!("Unknown host: {}", msg),
            SshErrorKind::Other(msg) => msg.clone(),
        }
    }

    /// Get display color code for terminal rendering (used by UI)
    pub fn color_code(&self) -> &'static str {
        match self {
            SshErrorKind::HostKeyMismatch(_) | SshErrorKind::ConnectionFailed(_) => "red", // critical
            SshErrorKind::AuthenticationFailed(_) => "yellow", // user can retry
            SshErrorKind::AgentUnavailable(_) => "cyan",       // informational
            SshErrorKind::HostKeyUnknown(_) => "yellow",       // needs user action
            SshErrorKind::Other(_) => "gray",                  // neutral
        }
    }
}

/// State for the SSH connection form
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SshConnectionState {
    /// Raw input, e.g. "user@example.com:22"
    pub address: String,
    /// "password" or "key"
    pub auth_method: SshAuthMethod,
    /// Password (if method == Password) or path to private key file
    pub credential: String,
    /// Which field has cursor focus: Address, Credential
    pub focused_field: SshDialogField,
    /// Error from last failed attempt (categorized)
    pub error: Option<SshErrorKind>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq, Default)]
pub enum SshAuthMethod {
    #[default]
    Password,
    KeyFile,
    Agent,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq, Default)]
pub enum SshDialogField {
    #[default]
    Address,
    Credential,
}

impl Default for SshConnectionState {
    fn default() -> Self {
        Self {
            address: String::new(),
            auth_method: SshAuthMethod::Password,
            credential: String::new(),
            focused_field: SshDialogField::Address,
            error: None,
        }
    }
}
