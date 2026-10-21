use baton_core::sysproxy::SysProxyError;
use baton_core::{ConfigError, DialError, HandshakeError, ProbeError};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: &'static str,
    pub hop: Option<usize>,
    pub detail: String,
}

impl CommandError {
    pub fn new(code: &'static str, detail: impl ToString) -> Self {
        Self {
            code,
            hop: None,
            detail: detail.to_string(),
        }
    }

    pub fn at(mut self, hop: usize) -> Self {
        self.hop = Some(hop);
        self
    }

    pub fn invalid_hop(hop: usize, error: ConfigError) -> Self {
        let code = match error {
            ConfigError::EmptyHost => "empty_host",
            ConfigError::ZeroPort => "invalid_port",
            ConfigError::CredentialsTooLong => "credentials_too_long",
        };
        Self::new(code, error).at(hop)
    }
}

impl From<DialError> for CommandError {
    fn from(error: DialError) -> Self {
        let detail = error.to_string();
        match error {
            DialError::Empty => Self::new("empty_chain", detail),
            DialError::Timeout => Self::new("timeout", detail),
            DialError::Connect(_) => Self::new("hop_unreachable", detail).at(0),
            DialError::Handshake { hop, source } => {
                let code = match source {
                    HandshakeError::AuthFailed => "auth_failed",
                    HandshakeError::AuthRequired => "auth_required",
                    HandshakeError::HttpStatus(_) | HandshakeError::Socks5Reply(_) => {
                        "hop_rejected"
                    }
                    HandshakeError::Io(_) | HandshakeError::Malformed => "protocol_error",
                    HandshakeError::InvalidInput(_) => "invalid_input",
                };
                Self::new(code, detail).at(hop)
            }
        }
    }
}

impl From<ProbeError> for CommandError {
    fn from(error: ProbeError) -> Self {
        match error {
            ProbeError::Dial(error) => error.into(),
            ProbeError::Timeout => Self::new("timeout", error),
            ProbeError::Lookup(_) => Self::new("lookup_failed", error),
        }
    }
}

impl From<SysProxyError> for CommandError {
    fn from(error: SysProxyError) -> Self {
        Self::new("system_proxy_failed", error)
    }
}
