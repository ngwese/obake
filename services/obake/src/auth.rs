// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! `obake auth` commands: SSH enrollment, device approval, rotation.

use crate::config::{load_config, runtime_base, save_config, HostConfig};
use crate::{Client, ClientError, Endpoint, Resolved};
use serde::Deserialize;
use std::path::Path;
use std::process::{Command, Stdio};

/// Login or credential errors.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct AuthError(String);

/// `obake auth login <ssh-user@host>`.
pub fn login(config_path: &Path, target: &str) -> Result<(), AuthError> {
    let host = target
        .split_once('@')
        .map(|(_, host)| host)
        .filter(|h| !h.is_empty())
        .ok_or_else(|| AuthError(format!("expected user@host, got {target}")))?;
    let cred = ssh_credential(target)?;
    let mut cfg = load_config(config_path).map_err(|e| AuthError(e.to_string()))?;
    if cfg.default.is_none() {
        cfg.default = Some(host.to_string());
    }
    cfg.hosts.insert(
        host.to_string(),
        HostConfig {
            endpoint: format!("https://{host}:{}", cred.port),
            token: cred.token,
            fingerprint: Some(cred.fingerprint),
        },
    );
    save_config(config_path, &cfg).map_err(|e| AuthError(e.to_string()))?;
    Ok(())
}

fn ssh_credential(target: &str) -> Result<RemoteCredential, AuthError> {
    let child = Command::new("ssh")
        .arg(target)
        .arg("PATH=\"$HOME/.local/bin:$PATH\" obake auth credential")
        .stdin(Stdio::inherit())
        .stderr(Stdio::inherit())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| AuthError(format!("ssh: {e}")))?;
    let output = child
        .wait_with_output()
        .map_err(|e| AuthError(format!("ssh: {e}")))?;
    if !output.status.success() {
        return Err(AuthError(format!("ssh exited {}", output.status)));
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|e| AuthError(format!("credential json: {e}")))
}

#[derive(Debug, Deserialize)]
struct RemoteCredential {
    token: String,
    port: u16,
    fingerprint: String,
}

/// Print the local socket credential JSON.
pub async fn print_credential() -> Result<(), AuthError> {
    let sock = runtime_base(None).join("obake-session/api.sock");
    let client = Client::new(Endpoint::Unix {
        path: sock.display().to_string(),
    });
    let bytes = client
        .get_raw("/api/v0/credential")
        .await
        .map_err(|e| AuthError(e.to_string()))?;
    let mut text = String::from_utf8(bytes).map_err(|e| AuthError(e.to_string()))?;
    if !text.ends_with('\n') {
        text.push('\n');
    }
    print!("{text}");
    Ok(())
}

/// Approve a device code on the resolved endpoint.
pub async fn approve(resolved: &Resolved, code: &str) -> Result<(), ClientError> {
    Client::from_resolved(resolved).approve_login(code).await
}

/// Rotate the bearer and store it on the selected config host.
pub async fn rotate(config_path: &Path, resolved: &Resolved) -> Result<(), AuthError> {
    let new_token = Client::from_resolved(resolved)
        .rotate_auth()
        .await
        .map_err(|e| AuthError(e.to_string()))?;
    if let Some(name) = &resolved.host_name {
        let mut cfg = load_config(config_path).map_err(|e| AuthError(e.to_string()))?;
        if let Some(host) = cfg.hosts.get_mut(name) {
            host.token = new_token;
            save_config(config_path, &cfg).map_err(|e| AuthError(e.to_string()))?;
        }
    }
    Ok(())
}
