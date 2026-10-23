use std::process::Command;

use serde::{Deserialize, Serialize};

use super::{ProxyEndpoints, SystemProxyError, domain_patterns};
use crate::bypass_list::{BypassList, Network};
use crate::hop::Hop;

const PROXY_KINDS: [&str; 3] = ["webproxy", "securewebproxy", "socksfirewallproxy"];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Snapshot {
    services: Vec<ServiceState>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ServiceState {
    name: String,
    proxies: Vec<ProxyState>,
    bypass: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ProxyState {
    kind: String,
    enabled: bool,
    server: String,
    port: u16,
}

pub(super) fn snapshot() -> Result<Snapshot, SystemProxyError> {
    let mut services = Vec::new();
    for name in network_services()? {
        let mut proxies = Vec::with_capacity(PROXY_KINDS.len());
        for kind in PROXY_KINDS {
            let output = networksetup(&[&format!("-get{kind}"), &name])?;
            proxies.push(parse_proxy_state(kind, &output));
        }
        let bypass = parse_bypass(&networksetup(&["-getproxybypassdomains", &name])?);
        services.push(ServiceState { name, proxies, bypass });
    }
    Ok(Snapshot { services })
}

pub(super) fn apply(host: &str, port: u16, bypass: &BypassList) -> Result<(), SystemProxyError> {
    let port = port.to_string();
    let networks = bypass.networks().iter().filter_map(network_pattern);
    let entries: Vec<String> = domain_patterns(bypass).chain(networks).collect();
    for name in network_services()? {
        for kind in PROXY_KINDS {
            networksetup(&[&format!("-set{kind}"), &name, host, &port])?;
        }
        let mut args = vec!["-setproxybypassdomains", name.as_str()];
        args.extend(entries.iter().map(String::as_str));
        networksetup(&args)?;
    }
    Ok(())
}

pub(super) fn restore(snapshot: &Snapshot) -> Result<(), SystemProxyError> {
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

pub(super) fn current() -> Result<Option<Hop>, SystemProxyError> {
    let output = Command::new("/usr/sbin/scutil").arg("--proxy").output()?;
    if !output.status.success() {
        return Err(SystemProxyError::Command {
            command: "scutil --proxy".into(),
            message: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }
    Ok(parse_scutil(&String::from_utf8_lossy(&output.stdout)).preferred_hop())
}

pub(super) fn endpoints(snapshot: &Snapshot) -> ProxyEndpoints {
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

fn network_pattern(network: &Network) -> Option<String> {
    if network.is_host() {
        Some(network.ip().to_string())
    } else if network.ip().is_ipv4() {
        Some(format!("{}/{}", network.ip(), network.prefix_len()))
    } else {
        None
    }
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
