use serde::{Deserialize, Serialize};

use crate::address::TargetAddr;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProxyProtocol {
    Http,
    #[default]
    Socks5,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hop {
    pub protocol: ProxyProtocol,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
}

#[derive(Debug, thiserror::Error)]
pub enum HopError {
    #[error("host is empty")]
    EmptyHost,
    #[error("port is zero")]
    ZeroPort,
    #[error("username or password is longer than 255 bytes")]
    CredentialsTooLong,
}

impl Hop {
    pub(crate) fn address(&self) -> TargetAddr {
        TargetAddr::new(self.host.trim(), self.port)
    }

    pub fn credentials(&self) -> Option<(&str, &str)> {
        if self.username.is_empty() { None } else { Some((&self.username, &self.password)) }
    }

    pub fn validate(&self) -> Result<(), HopError> {
        if self.host.trim().is_empty() {
            return Err(HopError::EmptyHost);
        }
        if self.port == 0 {
            return Err(HopError::ZeroPort);
        }
        if self.protocol == ProxyProtocol::Socks5
            && (self.username.len() > 255 || self.password.len() > 255)
        {
            return Err(HopError::CredentialsTooLong);
        }
        Ok(())
    }
}
