// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Config, state, share, image, and runtime path layout.

use std::env;
use std::path::{Path, PathBuf};

/// Overrideable on-disk layout for the session daemon.
#[derive(Debug, Clone)]
pub struct Paths {
    /// `$XDG_CONFIG_HOME/obake-session` (host.yaml, rigs, token).
    pub config_dir: PathBuf,
    /// `$XDG_STATE_HOME/obake-session` (current, sessions).
    pub state_dir: PathBuf,
    /// `$XDG_DATA_HOME/obake-session` (rig and session work areas).
    pub share_dir: PathBuf,
    /// Default shape image directory (`~/shapes`).
    pub default_image_dir: PathBuf,
    /// `$XDG_RUNTIME_DIR` for the PipeWire socket bind and API socket.
    pub runtime_dir: PathBuf,
    /// Optional Ravenna config directory (milestone 5).
    pub ravenna_config_dir: PathBuf,
}

impl Paths {
    /// Resolve paths from the process environment (home / XDG).
    pub fn from_env() -> Self {
        let home = env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/home/obake"));
        let config_home = env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"));
        let state_home = env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local/state"));
        let data_home = env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local/share"));
        let runtime_dir = env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/run/user/1000"));
        Self {
            config_dir: config_home.join("obake-session"),
            state_dir: state_home.join("obake-session"),
            share_dir: data_home.join("obake-session"),
            default_image_dir: home.join("shapes"),
            runtime_dir,
            ravenna_config_dir: config_home.join("obake-ravenna"),
        }
    }

    /// Build an isolated layout under `root` for tests.
    pub fn under(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref();
        Self {
            config_dir: root.join("config"),
            state_dir: root.join("state"),
            share_dir: root.join("share"),
            default_image_dir: root.join("shapes"),
            runtime_dir: root.join("runtime"),
            ravenna_config_dir: root.join("ravenna"),
        }
    }

    /// Ensure config, state, share, and runtime directories exist.
    pub fn ensure(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(self.config_dir.join("rigs"))?;
        std::fs::create_dir_all(self.state_dir.join("sessions"))?;
        std::fs::create_dir_all(self.share_dir.join("rigs"))?;
        std::fs::create_dir_all(self.share_dir.join("sessions"))?;
        std::fs::create_dir_all(self.runtime_dir.join("obake-session"))?;
        Ok(())
    }

    /// `host.yaml` path.
    pub fn host_yaml(&self) -> PathBuf {
        self.config_dir.join("host.yaml")
    }

    /// Bearer token file for the TCP listener.
    pub fn token_file(&self) -> PathBuf {
        self.config_dir.join("token")
    }

    /// Rig document directory.
    pub fn rigs_dir(&self) -> PathBuf {
        self.config_dir.join("rigs")
    }

    /// One rig YAML.
    pub fn rig_file(&self, name: &str) -> PathBuf {
        self.rigs_dir().join(format!("{name}.yaml"))
    }

    /// `current` active-session marker.
    pub fn current_file(&self) -> PathBuf {
        self.state_dir.join("current")
    }

    /// Persisted session snapshot.
    pub fn session_file(&self, id: &str) -> PathBuf {
        self.state_dir.join("sessions").join(format!("{id}.yaml"))
    }

    /// Rig working directory (mounted at `/work`).
    pub fn rig_work(&self, name: &str) -> PathBuf {
        self.share_dir.join("rigs").join(name).join("work")
    }

    /// Session work area (mounted at `/session`).
    pub fn session_work(&self, id: &str) -> PathBuf {
        self.share_dir.join("sessions").join(id).join("work")
    }

    /// GraphQL Unix socket.
    pub fn api_socket(&self) -> PathBuf {
        self.runtime_dir.join("obake-session").join("api.sock")
    }

    /// Shape SIF path under `image_dir`.
    pub fn shape_image(&self, image_dir: &Path, shape: &str) -> PathBuf {
        image_dir.join(format!("{shape}.sif"))
    }
}
