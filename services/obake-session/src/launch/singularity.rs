// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Real `singularity instance` launcher.

use super::{LaunchError, LaunchRequest, Launcher};
use std::process::Command;

/// Launches shapes with the host `singularity` binary.
#[derive(Debug, Default)]
pub struct SingularityLauncher;

impl SingularityLauncher {
    /// Create a singularity launcher.
    pub fn new() -> Self {
        Self
    }
}

fn looks_like_relative_work_path(arg: &str) -> bool {
    !arg.is_empty()
        && !arg.starts_with('-')
        && !arg.starts_with('/')
        && !arg.starts_with("./")
        && !arg.contains("://")
}

impl Launcher for SingularityLauncher {
    fn start(&self, request: &LaunchRequest) -> Result<(), LaunchError> {
        // Prefer `instance run` so the image %runscript (e.g. chuck) executes.
        // `instance start` only runs %startscript, which is empty on current
        // shape images and leaves an idle sinit with no audio process.
        let mut cmd = Command::new("singularity");
        cmd.arg("instance").arg("run");
        for bind in &request.binds {
            let spec = if bind.readonly {
                format!("{}:{}:ro", bind.source.display(), bind.target)
            } else {
                format!("{}:{}", bind.source.display(), bind.target)
            };
            cmd.arg("--bind").arg(spec);
        }
        for (k, v) in &request.env {
            cmd.env(k, v);
        }
        cmd.arg(&request.image);
        cmd.arg(&request.singularity_name);
        for arg in &request.command {
            // Work files are mounted at /work; rewrite relative paths so the
            // runscript finds them (instance run has no --pwd).
            if looks_like_relative_work_path(arg) {
                cmd.arg(format!("/work/{arg}"));
            } else {
                cmd.arg(arg);
            }
        }
        let status = cmd
            .status()
            .map_err(|e| LaunchError::Message(e.to_string()))?;
        if !status.success() {
            return Err(LaunchError::Message(format!(
                "singularity instance run failed: {status}"
            )));
        }
        Ok(())
    }

    fn stop(&self, singularity_name: &str) -> Result<(), LaunchError> {
        let status = Command::new("singularity")
            .arg("instance")
            .arg("stop")
            .arg(singularity_name)
            .status()
            .map_err(|e| LaunchError::Message(e.to_string()))?;
        if !status.success() {
            return Err(LaunchError::Message(format!(
                "singularity instance stop failed: {status}"
            )));
        }
        Ok(())
    }
}
