// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Config, state, share, image, and runtime path layout.

use std::env;
use std::path::{Path, PathBuf};

/// How an installed rig is stored under the config rigs tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RigPackage {
    /// `rigs/<name>.yaml`
    File {
        /// Path to the YAML document.
        path: PathBuf,
    },
    /// `rigs/<name>/rig.yaml` with optional `work/` template.
    Dir {
        /// Path to `rig.yaml`.
        definition: PathBuf,
        /// Path to the optional package `work/` template directory.
        work: PathBuf,
    },
}

impl RigPackage {
    /// Path to the YAML definition for either form.
    pub fn definition(&self) -> &Path {
        match self {
            Self::File { path } => path,
            Self::Dir { definition, .. } => definition,
        }
    }

    /// Package `work/` template directory, when this is a directory package.
    pub fn work_template(&self) -> Option<&Path> {
        match self {
            Self::File { .. } => None,
            Self::Dir { work, .. } => Some(work.as_path()),
        }
    }
}

/// Overrideable on-disk layout for the session daemon.
#[derive(Debug, Clone)]
pub struct Paths {
    /// `$XDG_CONFIG_HOME/obake-session` (host.yaml, rigs, token).
    pub config_dir: PathBuf,
    /// `$XDG_STATE_HOME/obake-session` (current, sessions).
    pub state_dir: PathBuf,
    /// `$XDG_DATA_HOME/obake-session` (session work areas).
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
        std::fs::create_dir_all(self.share_dir.join("sessions"))?;
        std::fs::create_dir_all(self.runtime_dir.join("obake-session"))?;
        Ok(())
    }

    /// `host.yaml` path.
    pub fn host_yaml(&self) -> PathBuf {
        self.config_dir.join("host.yaml")
    }

    /// Bearer token file for the TLS listener.
    pub fn token_file(&self) -> PathBuf {
        self.config_dir.join("token")
    }

    /// PEM certificate for the TLS listener.
    pub fn tls_cert_file(&self) -> PathBuf {
        self.config_dir.join("tls.crt")
    }

    /// PEM private key for the TLS listener.
    pub fn tls_key_file(&self) -> PathBuf {
        self.config_dir.join("tls.key")
    }

    /// Rig package directory.
    pub fn rigs_dir(&self) -> PathBuf {
        self.config_dir.join("rigs")
    }

    /// Single-file form path: `rigs/<name>.yaml`.
    pub fn rig_file(&self, name: &str) -> PathBuf {
        self.rigs_dir().join(format!("{name}.yaml"))
    }

    /// Directory form root: `rigs/<name>/`.
    pub fn rig_dir(&self, name: &str) -> PathBuf {
        self.rigs_dir().join(name)
    }

    /// Directory form definition: `rigs/<name>/rig.yaml`.
    pub fn rig_dir_definition(&self, name: &str) -> PathBuf {
        self.rig_dir(name).join("rig.yaml")
    }

    /// Directory form work template: `rigs/<name>/work/`.
    pub fn rig_package_work(&self, name: &str) -> PathBuf {
        self.rig_dir(name).join("work")
    }

    /// True when both file and directory forms exist for `name`.
    pub fn rig_name_conflict(&self, name: &str) -> bool {
        self.rig_file(name).is_file() && self.rig_dir_definition(name).is_file()
    }

    /// Resolve the installed package for `name`, if any.
    ///
    /// Returns `Err(())` when both forms exist (conflict).
    pub fn resolve_rig(&self, name: &str) -> Result<Option<RigPackage>, ()> {
        let file = self.rig_file(name);
        let def = self.rig_dir_definition(name);
        let has_file = file.is_file();
        let has_dir = def.is_file();
        match (has_file, has_dir) {
            (true, true) => Err(()),
            (true, false) => Ok(Some(RigPackage::File { path: file })),
            (false, true) => Ok(Some(RigPackage::Dir {
                definition: def,
                work: self.rig_package_work(name),
            })),
            (false, false) => Ok(None),
        }
    }

    /// `current` active-session marker.
    pub fn current_file(&self) -> PathBuf {
        self.state_dir.join("current")
    }

    /// Persisted session snapshot.
    pub fn session_file(&self, id: &str) -> PathBuf {
        self.state_dir.join("sessions").join(format!("{id}.yaml"))
    }

    /// Session work directory (mounted at `/work`).
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn resolves_file_and_dir_forms() {
        let dir = tempdir().unwrap();
        let paths = Paths::under(dir.path());
        paths.ensure().unwrap();
        fs::write(paths.rig_file("duo"), "instances: {}\nlinks: []\n").unwrap();
        fs::create_dir_all(paths.rig_dir("tone")).unwrap();
        fs::write(paths.rig_dir_definition("tone"), "instances: {}\nlinks: []\n").unwrap();
        fs::create_dir_all(paths.rig_package_work("tone")).unwrap();
        fs::write(paths.rig_package_work("tone").join("tone.ck"), "x").unwrap();

        let duo = paths.resolve_rig("duo").unwrap().unwrap();
        assert!(matches!(duo, RigPackage::File { .. }));
        let tone = paths.resolve_rig("tone").unwrap().unwrap();
        match tone {
            RigPackage::Dir { work, .. } => {
                assert!(work.join("tone.ck").is_file());
            }
            _ => panic!("expected dir"),
        }
        assert!(paths.resolve_rig("missing").unwrap().is_none());
    }

    #[test]
    fn detects_name_conflict() {
        let dir = tempdir().unwrap();
        let paths = Paths::under(dir.path());
        paths.ensure().unwrap();
        fs::write(paths.rig_file("tone"), "instances: {}\nlinks: []\n").unwrap();
        fs::create_dir_all(paths.rig_dir("tone")).unwrap();
        fs::write(paths.rig_dir_definition("tone"), "instances: {}\nlinks: []\n").unwrap();
        assert!(paths.rig_name_conflict("tone"));
        assert!(paths.resolve_rig("tone").is_err());
    }
}
