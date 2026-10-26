// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Client host config (`~/.config/obake/config.yaml`).

use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// One named host.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct HostConfig {
    /// GraphQL HTTP endpoint (or `unix://` path).
    pub endpoint: String,
    /// Bearer token for TCP endpoints.
    pub token: String,
}

/// Multi-host client configuration.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Default)]
pub struct ClientConfig {
    /// Default host name.
    #[serde(default)]
    pub default: Option<String>,
    /// Named hosts.
    #[serde(default)]
    pub hosts: HashMap<String, HostConfig>,
}

/// Resolution / config errors.
#[derive(Debug, Error)]
pub enum ResolveError {
    /// Unknown host name.
    #[error("unknown host `{0}`")]
    UnknownHost(String),
    /// Default host missing from hosts map.
    #[error("default host `{0}` is not in hosts")]
    BadDefault(String),
    /// I/O.
    #[error("config i/o: {0}")]
    Io(#[from] std::io::Error),
    /// YAML.
    #[error("config parse: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

/// Default config path from the environment.
pub fn default_config_path() -> PathBuf {
    if let Ok(xdg) = env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg).join("obake/config.yaml");
    }
    let home = env::var("HOME").unwrap_or_else(|_| "/home/obake".into());
    PathBuf::from(home).join(".config/obake/config.yaml")
}

/// Load config; missing file yields empty defaults.
pub fn load_config(path: &Path) -> Result<ClientConfig, ResolveError> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(serde_yaml::from_str(&text)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(ClientConfig::default()),
        Err(e) => Err(e.into()),
    }
}

/// Resolved connection target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    /// Endpoint URL or unix socket path marked as `unix:`.
    pub endpoint: String,
    /// Optional bearer token.
    pub token: Option<String>,
}

/// Resolve endpoint/token.
///
/// Order: `--endpoint`/`--token` > `--host` > config `default` > local unix socket.
pub fn resolve_endpoint(
    config: &ClientConfig,
    host: Option<&str>,
    endpoint: Option<&str>,
    token: Option<&str>,
    runtime_dir: Option<&Path>,
) -> Result<Resolved, ResolveError> {
    if endpoint.is_some() || (token.is_some() && host.is_none() && endpoint.is_some()) {
        // Explicit endpoint wins; token may come from flag or selected host.
    }

    if let Some(ep) = endpoint {
        let tok = if let Some(t) = token {
            Some(t.to_string())
        } else if let Some(h) = host.or(config.default.as_deref()) {
            config.hosts.get(h).map(|c| c.token.clone())
        } else {
            None
        };
        return Ok(Resolved {
            endpoint: ep.to_string(),
            token: tok.or_else(|| token.map(str::to_string)),
        });
    }

    let selected = if let Some(h) = host {
        if !config.hosts.contains_key(h) {
            return Err(ResolveError::UnknownHost(h.into()));
        }
        Some(h.to_string())
    } else if let Some(d) = &config.default {
        if !config.hosts.contains_key(d) {
            return Err(ResolveError::BadDefault(d.clone()));
        }
        Some(d.clone())
    } else {
        None
    };

    if let Some(name) = selected {
        let h = &config.hosts[&name];
        return Ok(Resolved {
            endpoint: h.endpoint.clone(),
            token: Some(token.unwrap_or(&h.token).to_string()),
        });
    }

    // Local unix socket
    let runtime = runtime_dir
        .map(PathBuf::from)
        .or_else(|| env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("/run/user/1000"));
    let sock = runtime.join("obake-session/api.sock");
    Ok(Resolved {
        endpoint: format!("unix://{}", sock.display()),
        token: token.map(str::to_string),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn resolution_order() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.yaml");
        fs::write(
            &path,
            r#"
default: studio
hosts:
  studio:
    endpoint: http://192.168.60.10:9477
    token: tok-studio
  spare:
    endpoint: http://192.168.60.11:9477
    token: tok-spare
"#,
        )
        .unwrap();
        let cfg = load_config(&path).unwrap();

        let r = resolve_endpoint(&cfg, None, None, None, None).unwrap();
        assert_eq!(r.endpoint, "http://192.168.60.10:9477");
        assert_eq!(r.token.as_deref(), Some("tok-studio"));

        let r = resolve_endpoint(&cfg, Some("spare"), None, None, None).unwrap();
        assert_eq!(r.endpoint, "http://192.168.60.11:9477");
        assert_eq!(r.token.as_deref(), Some("tok-spare"));

        let r = resolve_endpoint(
            &cfg,
            None,
            Some("http://override:1"),
            None,
            None,
        )
        .unwrap();
        assert_eq!(r.endpoint, "http://override:1");

        assert!(resolve_endpoint(&cfg, Some("missing"), None, None, None).is_err());

        let empty = ClientConfig::default();
        let r = resolve_endpoint(&empty, None, None, None, Some(dir.path())).unwrap();
        assert!(r.endpoint.starts_with("unix://"));
    }
}
