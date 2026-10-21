use std::process::Command;

use serde::{Deserialize, Serialize};

use super::{Endpoints, SysProxyError};
use crate::config::Hop;

const KEYS: &[(&str, &str)] = &[
    ("org.gnome.system.proxy", "mode"),
    ("org.gnome.system.proxy", "ignore-hosts"),
    ("org.gnome.system.proxy.http", "host"),
    ("org.gnome.system.proxy.http", "port"),
    ("org.gnome.system.proxy.https", "host"),
    ("org.gnome.system.proxy.https", "port"),
    ("org.gnome.system.proxy.socks", "host"),
    ("org.gnome.system.proxy.socks", "port"),
];

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    values: Vec<(String, String, String)>,
}

pub fn snapshot() -> Result<Snapshot, SysProxyError> {
    let mut values = Vec::with_capacity(KEYS.len());
    for (schema, key) in KEYS {
        let value = gsettings(&["get", schema, key])?;
        values.push((schema.to_string(), key.to_string(), value.trim().to_owned()));
    }
    Ok(Snapshot { values })
}

pub fn current() -> Result<Option<Hop>, SysProxyError> {
    Ok(endpoints(&snapshot()?).into_hop())
}

pub fn endpoints(snapshot: &Snapshot) -> Endpoints {
    let value = |schema: &str, key: &str| {
        snapshot
            .values
            .iter()
            .find(|(s, k, _)| s == schema && k == key)
            .map(|(_, _, value)| value.trim_matches('\'').to_owned())
            .unwrap_or_default()
    };
    if value("org.gnome.system.proxy", "mode") != "manual" {
        return Endpoints::default();
    }
    let endpoint = |schema: &str| {
        let host = value(schema, "host");
        let port = value(schema, "port")
            .parse()
            .ok()
            .filter(|port| *port != 0)?;
        (!host.is_empty()).then_some((host, port))
    };
    Endpoints {
        https: endpoint("org.gnome.system.proxy.https"),
        socks: endpoint("org.gnome.system.proxy.socks"),
        http: endpoint("org.gnome.system.proxy.http"),
    }
}

pub fn apply(host: &str, port: u16, bypass: &[&str]) -> Result<(), SysProxyError> {
    let port = port.to_string();
    for schema in [
        "org.gnome.system.proxy.http",
        "org.gnome.system.proxy.https",
        "org.gnome.system.proxy.socks",
    ] {
        gsettings(&["set", schema, "host", host])?;
        gsettings(&["set", schema, "port", &port])?;
    }
    let hosts = bypass
        .iter()
        .map(|host| format!("'{host}'"))
        .collect::<Vec<_>>()
        .join(", ");
    gsettings(&[
        "set",
        "org.gnome.system.proxy",
        "ignore-hosts",
        &format!("[{hosts}]"),
    ])?;
    gsettings(&["set", "org.gnome.system.proxy", "mode", "manual"])?;
    Ok(())
}

pub fn restore(snapshot: &Snapshot) -> Result<(), SysProxyError> {
    for (schema, key, value) in &snapshot.values {
        gsettings(&["set", schema, key, value])?;
    }
    Ok(())
}

fn gsettings(args: &[&str]) -> Result<String, SysProxyError> {
    let output = Command::new("gsettings").args(args).output()?;
    if !output.status.success() {
        return Err(SysProxyError::Command {
            command: format!("gsettings {}", args.join(" ")),
            message: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
