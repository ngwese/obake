// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Client host config (`~/.config/obake/config.yaml`).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// One named host.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct HostConfig {
    /// GraphQL HTTP endpoint (or `unix://` path).
    pub endpoint: String,
    /// Bearer token for TCP endpoints.
    pub token: String,
    /// `sha256:` fingerprint of the host certificate. Required for HTTPS.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
}

/// Multi-host client configuration.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Default)]
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

/// Write config mode 0600, replacing any previous file.
pub fn save_config(path: &Path, config: &ClientConfig) -> Result<(), ResolveError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let text = serde_yaml::to_string(config)?;
    let tmp = path.with_extension("yaml.tmp");
    {
        let mut f = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&tmp)?;
        f.write_all(text.as_bytes())?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
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
    /// Certificate fingerprint for HTTPS endpoints.
    pub fingerprint: Option<String>,
    /// Config host name this resolution came from, when one was selected.
    pub host_name: Option<String>,
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
        let fp = host
            .or(config.default.as_deref())
            .and_then(|h| config.hosts.get(h))
            .and_then(|c| c.fingerprint.clone());
        return Ok(Resolved {
            endpoint: ep.to_string(),
            token: tok.or_else(|| token.map(str::to_string)),
            fingerprint: fp,
            host_name: host.map(str::to_string).or_else(|| config.default.clone()),
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
            fingerprint: h.fingerprint.clone(),
            host_name: Some(name),
        });
    }

    // Local unix socket
    let runtime = runtime_base(runtime_dir);
    let sock = runtime.join("obake-session/api.sock");
    Ok(Resolved {
        endpoint: format!("unix://{}", sock.display()),
        token: token.map(str::to_string),
        fingerprint: None,
        host_name: None,
    })
}

/// Message when a command has no client config and the local socket is absent.
///
/// A missing config still uses the Unix socket when that socket exists, so a
/// command on the host keeps working. A laptop with neither file nor socket
/// is told how to enroll.
pub fn missing_remote_setup(config_path: &Path, resolved: &Resolved) -> Option<&'static str> {
    if config_path.exists() {
        return None;
    }
    let Some(path) = resolved.endpoint.strip_prefix("unix://") else {
        return None;
    };
    if Path::new(path).exists() {
        return None;
    }
    Some(
        "No client config and no local obake-session socket.\n\
         Use `obake auth login <ssh-user@host>` to set up remote access.",
    )
}

/// Runtime directory for the API socket.
///
/// Uses `XDG_RUNTIME_DIR` when set. A non-interactive SSH command often
/// leaves that unset; linger still creates `/run/user/<uid>`.
pub fn runtime_base(explicit: Option<&Path>) -> PathBuf {
    let xdg = env::var("XDG_RUNTIME_DIR").ok();
    runtime_from(explicit, xdg.as_deref(), euid())
}

pub fn runtime_from(explicit: Option<&Path>, xdg: Option<&str>, uid: u32) -> PathBuf {
    if let Some(dir) = explicit {
        return dir.to_path_buf();
    }
    if let Some(xdg) = xdg {
        return PathBuf::from(xdg);
    }
    PathBuf::from(format!("/run/user/{uid}"))
}

fn euid() -> u32 {
    unsafe { libc::geteuid() }
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

    #[test]
    fn missing_config_without_socket_hints_login() {
        let dir = tempdir().unwrap();
        let cfg_path = dir.path().join("config.yaml");
        let runtime = dir.path().join("run");
        let empty = ClientConfig::default();
        let resolved = resolve_endpoint(&empty, None, None, None, Some(&runtime)).unwrap();
        let hint = missing_remote_setup(&cfg_path, &resolved).unwrap();
        assert!(hint.contains("obake auth login <ssh-user@host>"));
    }

    #[test]
    fn missing_config_with_socket_has_no_hint() {
        let dir = tempdir().unwrap();
        let cfg_path = dir.path().join("config.yaml");
        let runtime = dir.path().join("run");
        fs::create_dir_all(runtime.join("obake-session")).unwrap();
        fs::write(runtime.join("obake-session/api.sock"), b"").unwrap();
        let empty = ClientConfig::default();
        let resolved = resolve_endpoint(&empty, None, None, None, Some(&runtime)).unwrap();
        assert!(missing_remote_setup(&cfg_path, &resolved).is_none());
    }

    #[test]
    fn existing_config_file_has_no_login_hint() {
        let dir = tempdir().unwrap();
        let cfg_path = dir.path().join("config.yaml");
        fs::write(&cfg_path, "hosts: {}\n").unwrap();
        let empty = ClientConfig::default();
        let resolved = resolve_endpoint(&empty, None, None, None, Some(dir.path())).unwrap();
        assert!(missing_remote_setup(&cfg_path, &resolved).is_none());
    }

    #[test]
    fn socket_dir_without_xdg_uses_uid() {
        assert_eq!(
            runtime_from(None, None, 42),
            PathBuf::from("/run/user/42")
        );
        assert_eq!(
            runtime_from(None, Some("/run/user/7"), 42),
            PathBuf::from("/run/user/7")
        );
    }
}
