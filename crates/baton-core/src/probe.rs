use std::io;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use rustls::pki_types::ServerName;
use serde::Serialize;
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_rustls::TlsConnector;

use crate::address::TargetAddr;
use crate::config::Hop;
use crate::dial::{DialError, dial};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EgressInfo {
    pub ip: String,
    pub country: String,
    pub region: String,
    pub city: String,
    pub org: String,
    pub latency_ms: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    #[error(transparent)]
    Dial(#[from] DialError),
    #[error("lookup service failed: {0}")]
    Lookup(String),
    #[error("timed out")]
    Timeout,
}

struct Lookup {
    host: &'static str,
    path: &'static str,
    normalize: fn(&Value) -> EgressInfo,
}

const LOOKUPS: &[Lookup] = &[
    Lookup {
        host: "ipinfo.io",
        path: "/json",
        normalize: normalize_ipinfo,
    },
    Lookup {
        host: "api.ip2location.io",
        path: "/",
        normalize: normalize_ip2location,
    },
];

pub async fn probe_egress(hops: &[Hop], timeout: Duration) -> Result<EgressInfo, ProbeError> {
    let mut last = None;
    for lookup in LOOKUPS {
        match tokio::time::timeout(timeout, fetch(hops, lookup)).await {
            Ok(Ok(info)) => return Ok(info),
            Ok(Err(error @ ProbeError::Dial(DialError::Connect(_)))) => return Err(error),
            Ok(Err(error)) => last = Some(error),
            Err(_) => last = Some(ProbeError::Timeout),
        }
    }
    Err(last.unwrap_or(ProbeError::Timeout))
}

async fn fetch(hops: &[Hop], lookup: &Lookup) -> Result<EgressInfo, ProbeError> {
    let started = Instant::now();
    let stream = dial(hops, &TargetAddr::Domain(lookup.host.into(), 443)).await?;

    let server_name =
        ServerName::try_from(lookup.host).map_err(|error| ProbeError::Lookup(error.to_string()))?;
    let mut tls = TlsConnector::from(tls_config())
        .connect(server_name, stream)
        .await
        .map_err(|error| ProbeError::Lookup(error.to_string()))?;
    let request = format!(
        "GET {} HTTP/1.0\r\nHost: {}\r\nAccept: application/json\r\nUser-Agent: Baton\r\n\r\n",
        lookup.path, lookup.host
    );
    tls.write_all(request.as_bytes())
        .await
        .map_err(|error| ProbeError::Lookup(error.to_string()))?;

    let mut response = Vec::new();
    if let Err(error) = tls.read_to_end(&mut response).await {
        if error.kind() != io::ErrorKind::UnexpectedEof || response.is_empty() {
            return Err(ProbeError::Lookup(error.to_string()));
        }
    }

    let body = parse_response(&response)?;
    let value: Value =
        serde_json::from_slice(body).map_err(|error| ProbeError::Lookup(error.to_string()))?;
    let mut info = (lookup.normalize)(&value);
    if info.ip.is_empty() {
        return Err(ProbeError::Lookup("response has no IP".into()));
    }
    info.latency_ms = started.elapsed().as_millis() as u64;
    Ok(info)
}

fn parse_response(response: &[u8]) -> Result<&[u8], ProbeError> {
    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut parsed = httparse::Response::new(&mut headers);
    let head_len = match parsed.parse(response) {
        Ok(httparse::Status::Complete(len)) => len,
        _ => return Err(ProbeError::Lookup("malformed HTTP response".into())),
    };
    match parsed.code {
        Some(200) => Ok(&response[head_len..]),
        code => Err(ProbeError::Lookup(format!("HTTP {}", code.unwrap_or(0)))),
    }
}

fn tls_config() -> Arc<rustls::ClientConfig> {
    static CONFIG: OnceLock<Arc<rustls::ClientConfig>> = OnceLock::new();
    CONFIG
        .get_or_init(|| {
            let roots = rustls::RootCertStore {
                roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
            };
            let config = rustls::ClientConfig::builder_with_provider(Arc::new(
                rustls::crypto::ring::default_provider(),
            ))
            .with_safe_default_protocol_versions()
            .expect("ring supports the default protocol versions")
            .with_root_certificates(roots)
            .with_no_client_auth();
            Arc::new(config)
        })
        .clone()
}

fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn normalize_ipinfo(value: &Value) -> EgressInfo {
    EgressInfo {
        ip: text(value, "ip"),
        country: text(value, "country"),
        region: text(value, "region"),
        city: text(value, "city"),
        org: text(value, "org"),
        latency_ms: 0,
    }
}

fn normalize_ip2location(value: &Value) -> EgressInfo {
    let asn = text(value, "asn");
    let as_name = text(value, "as");
    EgressInfo {
        ip: text(value, "ip"),
        country: text(value, "country_code"),
        region: text(value, "region_name"),
        city: text(value, "city_name"),
        org: match (asn.is_empty(), as_name.is_empty()) {
            (false, false) => format!("AS{asn} {as_name}"),
            (true, false) => as_name,
            _ => String::new(),
        },
        latency_ms: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_successful_response() {
        let response =
            b"HTTP/1.0 200 OK\r\nContent-Type: application/json\r\n\r\n{\"ip\":\"1.2.3.4\"}";
        assert_eq!(parse_response(response).unwrap(), b"{\"ip\":\"1.2.3.4\"}");
    }

    #[test]
    fn rejects_error_status() {
        let response = b"HTTP/1.0 429 Too Many Requests\r\n\r\n";
        assert!(matches!(
            parse_response(response),
            Err(ProbeError::Lookup(_))
        ));
    }

    #[test]
    fn normalizes_ip2location() {
        let value = serde_json::json!({
            "ip": "1.2.3.4",
            "country_code": "US",
            "region_name": "California",
            "city_name": "San Jose",
            "asn": "13335",
            "as": "Cloudflare",
        });
        let info = normalize_ip2location(&value);
        assert_eq!(info.org, "AS13335 Cloudflare");
        assert_eq!(info.city, "San Jose");
    }
}
