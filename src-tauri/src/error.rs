use baton_core::system_proxy::SystemProxyError;
use baton_core::{DialError, HandshakeError, HopError, ProbeError};
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
        Self { code, hop: None, detail: detail.to_string() }
    }

    pub fn with_hop(mut self, hop: usize) -> Self {
        self.hop = Some(hop);
        self
    }

    pub fn invalid_hop(hop: usize, error: HopError) -> Self {
        let code = match error {
            HopError::EmptyHost => "empty_host",
            HopError::ZeroPort => "invalid_port",
            HopError::CredentialsTooLong => "credentials_too_long",
        };
        Self::new(code, error).with_hop(hop)
    }
}

impl CommandError {
    pub fn probe(error: ProbeError, hop_count: usize) -> Self {
        match error {
            ProbeError::Dial(error) => Self::dial(error, hop_count),
            ProbeError::Timeout => Self::new("timeout", error),
            ProbeError::Lookup(reason) => Self::new("lookup_failed", reason),
        }
    }

    fn dial(error: DialError, hop_count: usize) -> Self {
        let detail = error.to_string();
        match error {
            DialError::NoHops => Self::new("empty_chain", detail),
            DialError::Timeout => Self::new("timeout", detail),
            DialError::Unreachable(_) => Self::new("hop_unreachable", detail).with_hop(0),
            DialError::Handshake { hop, source } => {
                let code = match source {
                    HandshakeError::AuthFailed => "auth_failed",
                    HandshakeError::AuthRequired => "auth_required",
                    HandshakeError::HttpStatus(_) | HandshakeError::Socks5Reply(_)
                        if hop + 1 == hop_count =>
                    {
                        "target_unreachable"
                    }
                    HandshakeError::HttpStatus(_) | HandshakeError::Socks5Reply(_) => {
                        "hop_rejected"
                    }
                    HandshakeError::Io(_) | HandshakeError::Malformed => "protocol_error",
                    HandshakeError::InvalidInput(_) => "invalid_input",
                };
                Self::new(code, detail).with_hop(hop)
            }
        }
    }
}

impl From<SystemProxyError> for CommandError {
    fn from(error: SystemProxyError) -> Self {
        Self::new("system_proxy_failed", error)
    }
}
