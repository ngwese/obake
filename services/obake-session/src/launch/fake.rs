// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! In-memory launcher for tests.

use super::{LaunchError, LaunchRequest, Launcher};
use std::sync::Mutex;

/// Recorded start call.
pub type LaunchRecord = LaunchRequest;

/// Fake launcher that records start/stop calls.
#[derive(Debug, Default)]
pub struct FakeLauncher {
    /// Successful starts.
    pub started: Mutex<Vec<LaunchRequest>>,
    /// Stopped singularity names.
    pub stopped: Mutex<Vec<String>>,
}

impl FakeLauncher {
    /// Create an empty fake launcher.
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot of started requests.
    pub fn started(&self) -> Vec<LaunchRequest> {
        self.started.lock().unwrap().clone()
    }

    /// Snapshot of stopped names.
    pub fn stopped(&self) -> Vec<String> {
        self.stopped.lock().unwrap().clone()
    }
}

impl Launcher for FakeLauncher {
    fn start(&self, request: &LaunchRequest) -> Result<(), LaunchError> {
        self.started.lock().unwrap().push(request.clone());
        Ok(())
    }

    fn stop(&self, singularity_name: &str) -> Result<(), LaunchError> {
        self.stopped
            .lock()
            .unwrap()
            .push(singularity_name.to_string());
        Ok(())
    }
}
