use baton_core::{DialError, HandshakeError, HopError, ProbeError, SystemProxyError};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: &'static str,
    pub hop: Option<usize>,
    pub detail: String,
    pub reply: Option<u8>,
}

impl CommandError {
    pub fn new(code: &'static str, detail: impl ToString) -> Self {
        Self { code, hop: None, detail: detail.to_string(), reply: None }
    }

    pub fn with_hop(mut self, hop: usize) -> Self {
        self.hop = Some(hop);
        self
    }

    pub fn invalid_hop(hop: usize, error: HopError) -> Self {
        let code = match error {
            HopError::EmptyHost => "empty_host",
            HopError::InvalidPort => "invalid_port",
            HopError::CredentialsTooLong => "credentials_too_long",
        };
        Self::new(code, error).with_hop(hop)
    }

    pub fn probe(error: ProbeError, hop_count: usize) -> Self {
        match error {
            ProbeError::Dial(error) => Self::dial(error, hop_count),
            ProbeError::Timeout => Self::new("timeout", error),
            ProbeError::Lookup(reason) => Self::new("lookup_failed", reason),
        }
    }

    fn dial(error: DialError, hop_count: usize) -> Self {
        match error {
            DialError::EmptyChain => Self::new("empty_chain", error),
            DialError::Timeout => Self::new("timeout", error),
            DialError::HopUnreachable(_) => Self::new("hop_unreachable", error).with_hop(0),
            DialError::TargetUnreachable(_) => Self::new("target_unreachable", error),
            DialError::Handshake { hop, source } => Self::handshake(hop, source, hop_count),
        }
    }

    fn handshake(hop: usize, source: HandshakeError, hop_count: usize) -> Self {
        let code = match source {
            HandshakeError::AuthFailed => "auth_failed",
            HandshakeError::AuthRequired => "auth_required",
            HandshakeError::HttpStatus(_) | HandshakeError::Socks5Reply(_)
                if hop + 1 == hop_count =>
            {
                "target_unreachable"
            }
            HandshakeError::HttpStatus(_) | HandshakeError::Socks5Reply(_) => "hop_rejected",
            HandshakeError::Io(_) | HandshakeError::Malformed => "protocol_error",
            HandshakeError::InvalidInput(_) => "invalid_input",
        };
        let reply = match source {
            HandshakeError::Socks5Reply(reply) => Some(reply),
            _ => None,
        };
        Self { reply, ..Self::new(code, source).with_hop(hop) }
    }
}

impl From<SystemProxyError> for CommandError {
    fn from(error: SystemProxyError) -> Self {
        Self::new("system_proxy_failed", error)
    }
}
