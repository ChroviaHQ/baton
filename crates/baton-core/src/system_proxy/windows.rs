use std::io;
use std::ptr;

use serde::{Deserialize, Serialize};
use windows_sys::Win32::Networking::WinInet::{
    INTERNET_OPTION_REFRESH, INTERNET_OPTION_SETTINGS_CHANGED, InternetSetOptionW,
};
use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};

use super::{ProxyEndpoints, SystemProxyError, parse_windows_proxy_server};
use crate::config::Hop;

const INTERNET_SETTINGS: &str = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    proxy_enable: Option<u32>,
    proxy_server: Option<String>,
    proxy_override: Option<String>,
    auto_config_url: Option<String>,
}

pub fn snapshot() -> Result<Snapshot, SystemProxyError> {
    let key =
        RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(INTERNET_SETTINGS, KEY_READ)?;
    Ok(Snapshot {
        proxy_enable: key.get_value("ProxyEnable").ok(),
        proxy_server: key.get_value("ProxyServer").ok(),
        proxy_override: key.get_value("ProxyOverride").ok(),
        auto_config_url: key.get_value("AutoConfigURL").ok(),
    })
}

pub fn current() -> Result<Option<Hop>, SystemProxyError> {
    Ok(endpoints(&snapshot()?).preferred_hop())
}

pub fn endpoints(snapshot: &Snapshot) -> ProxyEndpoints {
    match (snapshot.proxy_enable, &snapshot.proxy_server) {
        (Some(1), Some(server)) => parse_windows_proxy_server(server),
        _ => ProxyEndpoints::default(),
    }
}

pub fn apply(host: &str, port: u16, bypass: &[&str]) -> Result<(), SystemProxyError> {
    let key = open_writable()?;
    let mut overrides = bypass.join(";");
    overrides.push_str(";<local>");
    key.set_value("ProxyEnable", &1u32)?;
    key.set_value("ProxyServer", &format!("{host}:{port}"))?;
    key.set_value("ProxyOverride", &overrides)?;
    delete_value(&key, "AutoConfigURL")?;
    refresh();
    Ok(())
}

pub fn restore(snapshot: &Snapshot) -> Result<(), SystemProxyError> {
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
