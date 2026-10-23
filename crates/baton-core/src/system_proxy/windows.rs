use std::io;
use std::net::IpAddr;
use std::ptr;

use serde::{Deserialize, Serialize};
use windows_sys::Win32::Networking::WinInet::{
    INTERNET_OPTION_REFRESH, INTERNET_OPTION_SETTINGS_CHANGED, InternetSetOptionW,
};
use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};

use super::{ProxyEndpoints, SystemProxyError, domain_patterns};
use crate::bypass_list::{BypassList, Network};
use crate::hop::Hop;

const INTERNET_SETTINGS: &str = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Snapshot {
    proxy_enable: Option<u32>,
    proxy_server: Option<String>,
    proxy_override: Option<String>,
    auto_config_url: Option<String>,
}

pub(super) fn snapshot() -> Result<Snapshot, SystemProxyError> {
    let key =
        RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(INTERNET_SETTINGS, KEY_READ)?;
    Ok(Snapshot {
        proxy_enable: key.get_value("ProxyEnable").ok(),
        proxy_server: key.get_value("ProxyServer").ok(),
        proxy_override: key.get_value("ProxyOverride").ok(),
        auto_config_url: key.get_value("AutoConfigURL").ok(),
    })
}

pub(super) fn current() -> Result<Option<Hop>, SystemProxyError> {
    Ok(endpoints(&snapshot()?).preferred_hop())
}

pub(super) fn endpoints(snapshot: &Snapshot) -> ProxyEndpoints {
    match (snapshot.proxy_enable, &snapshot.proxy_server) {
        (Some(1), Some(server)) => parse_proxy_server(server),
        _ => ProxyEndpoints::default(),
    }
}

pub(super) fn apply(host: &str, port: u16, bypass: &BypassList) -> Result<(), SystemProxyError> {
    let key = open_writable()?;
    let networks = bypass.networks().iter().flat_map(network_patterns);
    let mut overrides: Vec<String> = domain_patterns(bypass).chain(networks).collect();
    overrides.push("<local>".to_owned());
    key.set_value("ProxyEnable", &1u32)?;
    key.set_value("ProxyServer", &format!("{host}:{port}"))?;
    key.set_value("ProxyOverride", &overrides.join(";"))?;
    delete_value(&key, "AutoConfigURL")?;
    refresh();
    Ok(())
}

pub(super) fn restore(snapshot: &Snapshot) -> Result<(), SystemProxyError> {
    let key = open_writable()?;
    match snapshot.proxy_enable {
        Some(value) => key.set_value("ProxyEnable", &value)?,
        None => delete_value(&key, "ProxyEnable")?,
    }
    for (name, value) in [
        ("ProxyServer", &snapshot.proxy_server),
        ("ProxyOverride", &snapshot.proxy_override),
        ("AutoConfigURL", &snapshot.auto_config_url),
    ] {
        match value {
            Some(value) => key.set_value(name, value)?,
            None => delete_value(&key, name)?,
        }
    }
    refresh();
    Ok(())
}

fn open_writable() -> io::Result<RegKey> {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(INTERNET_SETTINGS, KEY_READ | KEY_WRITE)
}

fn delete_value(key: &RegKey, name: &str) -> io::Result<()> {
    match key.delete_value(name) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

fn refresh() {
    unsafe {
        InternetSetOptionW(ptr::null(), INTERNET_OPTION_SETTINGS_CHANGED, ptr::null(), 0);
        InternetSetOptionW(ptr::null(), INTERNET_OPTION_REFRESH, ptr::null(), 0);
    }
}

fn network_patterns(network: &Network) -> Vec<String> {
    let IpAddr::V4(ip) = network.ip() else {
        return if network.is_host() { vec![format!("[{}]", network.ip())] } else { Vec::new() };
    };
    let octets = network.prefix_len().div_ceil(8).max(1);
    let shift = 32 - octets * 8;
    let first = u32::from(ip) >> shift;
    (first..=first | ((1 << (octets * 8 - network.prefix_len())) - 1))
        .map(|prefix| {
            let bytes = (prefix << shift).to_be_bytes();
            let wildcard = (octets < 4).then(|| "*".to_owned());
            let parts: Vec<String> =
                bytes[..octets as usize].iter().map(u8::to_string).chain(wildcard).collect();
            parts.join(".")
        })
        .collect()
}

fn parse_endpoint(value: &str) -> Option<(String, u16)> {
    let value = value.trim();
    let value = value.split_once("://").map_or(value, |(_, rest)| rest);
    let value = value.trim_end_matches('/');
    let (host, port) = value.rsplit_once(':')?;
    let host = host.trim_start_matches('[').trim_end_matches(']');
    let port = port.parse().ok().filter(|port| *port != 0)?;
    (!host.is_empty()).then(|| (host.to_owned(), port))
}

fn parse_proxy_server(value: &str) -> ProxyEndpoints {
    if !value.contains('=') {
        let endpoint = parse_endpoint(value);
        return ProxyEndpoints { https: endpoint.clone(), http: endpoint, socks: None };
    }
    let mut endpoints = ProxyEndpoints::default();
    for entry in value.split(';') {
        let Some((scheme, address)) = entry.split_once('=') else {
            continue;
        };
        let endpoint = parse_endpoint(address);
        match scheme.trim().to_ascii_lowercase().as_str() {
            "https" => endpoints.https = endpoint,
            "http" => endpoints.http = endpoint,
            "socks" | "socks5" => endpoints.socks = endpoint,
            _ => {}
        }
    }
    endpoints
}
