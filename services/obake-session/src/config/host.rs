// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! `host.yaml` profile.

use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use thiserror::Error;

/// Errors loading or parsing the host profile.
#[derive(Debug, Error)]
pub enum HostProfileError {
    /// I/O failure.
    #[error("host.yaml i/o: {0}")]
    Io(#[from] std::io::Error),
    /// YAML parse failure.
    #[error("host.yaml parse: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

/// Optional read-only sample library bind.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SampleLibrary {
    /// Host path.
    pub source: String,
    /// Container path.
    pub target: String,
}

/// Extra bind mount from the host profile.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct HostBind {
    /// Host path.
    pub source: String,
    /// Container path.
    pub target: String,
    /// When true, mount read-only.
    #[serde(default)]
    pub readonly: bool,
}

/// Card alias (e.g. MixPre → ALSA card id `II`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Alias {
    /// ALSA card id string.
    pub card: String,
}

/// Host-wide launch and listener settings.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct HostProfile {
    /// TCP listen address (default `0.0.0.0:9477`).
    #[serde(default = "default_listen")]
    pub listen: String,
    /// Environment applied to every instance.
    #[serde(default)]
    pub environment: HashMap<String, String>,
    /// Extra binds applied to every instance.
    #[serde(default)]
    pub binds: Vec<HostBind>,
    /// Optional sample library (always read-only).
    #[serde(default)]
    pub sample_library: Option<SampleLibrary>,
    /// Card aliases for system links.
    #[serde(default)]
    pub aliases: HashMap<String, Alias>,
    /// Override for the shape image directory.
    #[serde(default)]
    pub image_dir: Option<String>,
}

fn default_listen() -> String {
    "0.0.0.0:9477".into()
}

impl Default for HostProfile {
    fn default() -> Self {
        Self {
            listen: default_listen(),
            environment: HashMap::new(),
            binds: Vec::new(),
            sample_library: None,
            aliases: HashMap::new(),
            image_dir: None,
        }
    }
}

/// Parse a host profile from YAML text.
pub fn parse_host(text: &str) -> Result<HostProfile, HostProfileError> {
    if text.trim().is_empty() {
        return Ok(HostProfile::default());
    }
    Ok(serde_yaml::from_str(text)?)
}

/// Load `host.yaml`, or defaults when the file is missing.
pub fn load_host(path: &Path) -> Result<HostProfile, HostProfileError> {
    match fs::read_to_string(path) {
        Ok(text) => parse_host(&text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(HostProfile::default()),
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_design_example() {
        let yaml = r#"
listen: "0.0.0.0:9477"
environment:
  OBAKE_HOST: qdl
binds: []
sample_library:
  source: /home/obake/samples
  target: /samples
aliases:
  mixpre:
    card: II
  adat:
    card: ADAT
"#;
        let host = parse_host(yaml).unwrap();
        assert_eq!(host.listen, "0.0.0.0:9477");
        assert_eq!(host.environment.get("OBAKE_HOST").unwrap(), "qdl");
        assert_eq!(host.aliases.get("mixpre").unwrap().card, "II");
        assert_eq!(
            host.sample_library.as_ref().unwrap().target,
            "/samples"
        );
    }
}
