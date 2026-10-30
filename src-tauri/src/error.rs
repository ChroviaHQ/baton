use baton_core::{DialError, HandshakeError, HopError, ProbeError, SystemProxyError};
use serde::Serialize;

use crate::relay::Chain;

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

    fn with_hop(mut self, hop: usize) -> Self {
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

    pub fn probe(error: ProbeError, chain: &Chain) -> Self {
        let error = match error {
            ProbeError::Dial(error) => Self::dial(error, chain.hops.len()),
            ProbeError::Timeout => Self::new("timeout", error),
            ProbeError::Lookup(reason) => Self::new("lookup_failed", reason),
        };
        Self { hop: error.hop.map(|hop| hop + chain.first_hop_index), ..error }
    }

    fn dial(error: DialError, hop_count: usize) -> Self {
        match error {
            DialError::Timeout => Self::new("timeout", error),
            DialError::HopUnreachable(_) => Self::new("hop_unreachable", error).with_hop(0),
            DialError::TargetUnreachable(source) => Self::new("direct_unreachable", source),
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
