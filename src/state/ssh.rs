use serde::{Deserialize, Serialize};

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
    /// Error message from last failed attempt
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq, Default)]
pub enum SshAuthMethod {
    #[default]
    Password,
    KeyFile,
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
