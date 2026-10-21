use std::process::Command;

use serde::{Deserialize, Serialize};

use super::{ProxyEndpoints, SystemProxyError};
use crate::config::Hop;

const KINDS: [&str; 3] = ["webproxy", "securewebproxy", "socksfirewallproxy"];

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    services: Vec<ServiceState>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct ServiceState {
    name: String,
    proxies: Vec<ProxyState>,
    bypass: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct ProxyState {
    kind: String,
    enabled: bool,
    server: String,
    port: u16,
}

pub fn snapshot() -> Result<Snapshot, SystemProxyError> {
    let mut services = Vec::new();
    for name in network_services()? {
        let mut proxies = Vec::with_capacity(KINDS.len());
        for kind in KINDS {
            let output = networksetup(&[&format!("-get{kind}"), &name])?;
            proxies.push(parse_proxy_state(kind, &output));
        }
        let bypass = parse_bypass(&networksetup(&["-getproxybypassdomains", &name])?);
        services.push(ServiceState { name, proxies, bypass });
    }
    Ok(Snapshot { services })
}

pub fn apply(host: &str, port: u16, bypass: &[&str]) -> Result<(), SystemProxyError> {
    let port = port.to_string();
    for name in network_services()? {
        for kind in KINDS {
            networksetup(&[&format!("-set{kind}"), &name, host, &port])?;
        }
        let mut args = vec!["-setproxybypassdomains", name.as_str()];
        args.extend_from_slice(bypass);
        networksetup(&args)?;
    }
    Ok(())
}

pub fn restore(snapshot: &Snapshot) -> Result<(), SystemProxyError> {
    let present = network_services()?;
    for service in &snapshot.services {
        if !present.contains(&service.name) {
            continue;
        }
        let name = service.name.as_str();
        for proxy in &service.proxies {
            if !proxy.server.is_empty() && proxy.port != 0 {
                networksetup(&[
                    &format!("-set{}", proxy.kind),
                    name,
                    &proxy.server,
                    &proxy.port.to_string(),
                ])?;
            }
            let state = if proxy.enabled { "on" } else { "off" };
            networksetup(&[&format!("-set{}state", proxy.kind), name, state])?;
        }
        let mut args = vec!["-setproxybypassdomains", name];
        if service.bypass.is_empty() {
            args.push("Empty");
        } else {
            args.extend(service.bypass.iter().map(String::as_str));
        }
        networksetup(&args)?;
    }
    Ok(())
}

pub fn current() -> Result<Option<Hop>, SystemProxyError> {
    let output = Command::new("/usr/sbin/scutil").arg("--proxy").output()?;
    if !output.status.success() {
        return Err(SystemProxyError::Command {
            command: "scutil --proxy".into(),
            message: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }
    Ok(parse_scutil(&String::from_utf8_lossy(&output.stdout)).preferred_hop())
}

pub fn endpoints(snapshot: &Snapshot) -> ProxyEndpoints {
    for service in &snapshot.services {
        let endpoint = |kind: &str| {
            service
                .proxies
                .iter()
                .find(|proxy| proxy.kind == kind && proxy.enabled && !proxy.server.is_empty())
                .map(|proxy| (proxy.server.clone(), proxy.port))
        };
        let endpoints = ProxyEndpoints {
            https: endpoint("securewebproxy"),
            socks: endpoint("socksfirewallproxy"),
            http: endpoint("webproxy"),
        };
        if endpoints != ProxyEndpoints::default() {
            return endpoints;
        }
    }
    ProxyEndpoints::default()
}

fn parse_scutil(output: &str) -> ProxyEndpoints {
    let field = |key: &str| {
        output.lines().find_map(|line| {
            let (name, value) = line.split_once(" : ")?;
            (name.trim() == key).then(|| value.trim().to_owned())
        })
    };
    let endpoint = |prefix: &str| {
        if field(&format!("{prefix}Enable")).as_deref() != Some("1") {
            return None;
        }
        let host = field(&format!("{prefix}Proxy"))?;
        let port = field(&format!("{prefix}Port"))?.parse().ok()?;
        Some((host, port))
    };
    ProxyEndpoints { https: endpoint("HTTPS"), socks: endpoint("SOCKS"), http: endpoint("HTTP") }
}

fn network_services() -> Result<Vec<String>, SystemProxyError> {
    let output = networksetup(&["-listallnetworkservices"])?;
    Ok(output
        .lines()
        .skip(1)
        .filter(|line| !line.is_empty() && !line.starts_with('*'))
        .map(str::to_owned)
        .collect())
}

fn networksetup(args: &[&str]) -> Result<String, SystemProxyError> {
    let output = Command::new("/usr/sbin/networksetup").args(args).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    if !output.status.success() || stdout.starts_with("** Error") {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(SystemProxyError::Command {
            command: format!("networksetup {}", args.first().copied().unwrap_or_default()),
            message: format!("{}{}", stdout.trim(), stderr.trim()),
        });
    }
    Ok(stdout)
}

fn parse_proxy_state(kind: &str, output: &str) -> ProxyState {
    let field = |key: &str| {
        output
            .lines()
            .find_map(|line| line.strip_prefix(key))
            .map(str::trim)
            .unwrap_or_default()
            .to_owned()
    };
    ProxyState {
        kind: kind.to_owned(),
        enabled: field("Enabled:") == "Yes",
        server: field("Server:"),
        port: field("Port:").parse().unwrap_or(0),
    }
}

fn parse_bypass(output: &str) -> Vec<String> {
    if output.contains("There aren't any bypass domains") {
        return Vec::new();
    }
    output.lines().map(str::trim).filter(|line| !line.is_empty()).map(str::to_owned).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_proxy_output() {
        let output =
            "Enabled: Yes\nServer: 127.0.0.1\nPort: 7890\nAuthenticated Proxy Enabled: 0\n";
        assert_eq!(
            parse_proxy_state("webproxy", output),
            ProxyState {
                kind: "webproxy".into(),
                enabled: true,
                server: "127.0.0.1".into(),
                port: 7890,
            }
        );
        let empty = parse_proxy_state("webproxy", "Enabled: No\nServer: \nPort: 0\n");
        assert!(!empty.enabled);
        assert!(empty.server.is_empty());
    }

    #[test]
    fn parses_scutil_output() {
        let output = "<dictionary> {\n  ExceptionsList : <array> {\n    0 : localhost\n  }\n  HTTPEnable : 1\n  HTTPPort : 8080\n  HTTPProxy : 127.0.0.1\n  HTTPSEnable : 0\n  SOCKSEnable : 1\n  SOCKSPort : 7891\n  SOCKSProxy : 127.0.0.1\n}\n";
        let hop = parse_scutil(output).preferred_hop().unwrap();
        assert_eq!((hop.protocol, hop.port), (crate::ProxyProtocol::Socks5, 7891));
        assert_eq!(parse_scutil("<dictionary> {\n  HTTPEnable : 0\n}\n").preferred_hop(), None);
    }

    #[test]
    fn parses_bypass_output() {
        assert!(parse_bypass("There aren't any bypass domains set on Wi-Fi.\n").is_empty());
        assert_eq!(parse_bypass("localhost\n*.local\n"), vec!["localhost", "*.local"]);
    }
}
