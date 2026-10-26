// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Shape instance launch via Singularity (or a fake for tests).

mod fake;
mod singularity;

pub use fake::{FakeLauncher, LaunchRecord};
pub use singularity::SingularityLauncher;

use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

/// Launch errors.
#[derive(Debug, Error)]
pub enum LaunchError {
    /// Underlying process or I/O failure.
    #[error("launch: {0}")]
    Message(String),
}

/// One bind mount for a shape instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindMount {
    /// Host path.
    pub source: PathBuf,
    /// Container path.
    pub target: String,
    /// Read-only when true.
    pub readonly: bool,
}

/// Arguments for starting one Singularity instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchRequest {
    /// Singularity instance name (`obake-<session>-<instance>`).
    pub singularity_name: String,
    /// Short instance / node name.
    pub instance_name: String,
    /// Shape name.
    pub shape: String,
    /// Path to the `.sif` image.
    pub image: PathBuf,
    /// Command arguments.
    pub command: Vec<String>,
    /// Environment variables (including `PIPEWIRE_PROPS`, `XDG_RUNTIME_DIR`).
    pub env: HashMap<String, String>,
    /// Bind mounts.
    pub binds: Vec<BindMount>,
}

/// Trait for starting and stopping shape instances.
pub trait Launcher: Send + Sync {
    /// Start one instance.
    fn start(&self, request: &LaunchRequest) -> Result<(), LaunchError>;
    /// Stop one Singularity instance by name.
    fn stop(&self, singularity_name: &str) -> Result<(), LaunchError>;
}

/// Build `PIPEWIRE_PROPS` JSON for a node name.
pub fn pipewire_props(node: &str) -> String {
    format!(
        "{{\"node.name\"=\"{node}\",\"client.name\"=\"{node}\",\"application.name\"=\"{node}\",\"node.description\"=\"{node}\",\"node.autoconnect\"=false}}"
    )
}

/// Singularity instance name for a session + instance.
pub fn singularity_name(session_id: &str, instance: &str) -> String {
    format!("obake-{session_id}-{instance}")
}
