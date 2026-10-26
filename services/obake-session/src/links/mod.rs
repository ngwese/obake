// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! PipeWire link reconcile via `pw-link` (or a fake for tests).

mod fake;

pub use fake::{FakeGraph, GraphPort, PortDirection};

use std::collections::{HashMap, HashSet};
use std::process::Command;
use thiserror::Error;

/// Link / graph errors.
#[derive(Debug, Error)]
pub enum LinkError {
    /// Underlying failure.
    #[error("link: {0}")]
    Message(String),
}

/// Desired or applied link state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkState {
    /// Ports not both present yet.
    Pending,
    /// Connected.
    Applied,
}

/// Status of one desired link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkStatus {
    /// `node:port` source.
    pub from: String,
    /// `node:port` destination.
    pub to: String,
    /// Pending or applied.
    pub state: LinkState,
}

/// Live graph operations used by the reconciler.
pub trait GraphOps: Send + Sync {
    /// List ports currently visible.
    fn list_ports(&self) -> Vec<GraphPort>;
    /// Connect two endpoints (`node:port`).
    fn connect(&self, from: &str, to: &str) -> Result<(), LinkError>;
    /// Disconnect two endpoints.
    fn disconnect(&self, from: &str, to: &str) -> Result<(), LinkError>;
    /// Resolve an ALSA card id to a PipeWire node name, if present.
    fn resolve_card(&self, card: &str) -> Option<String>;
}

/// Reconcile desired links against the live graph.
#[derive(Debug, Default)]
pub struct Reconciler {
    /// Currently applied links (from, to).
    applied: HashSet<(String, String)>,
}

impl Reconciler {
    /// Create an empty reconciler.
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply desired set: connect missing, disconnect removed.
    pub fn reconcile(
        &mut self,
        graph: &dyn GraphOps,
        desired: &[(String, String)],
    ) -> Result<Vec<LinkStatus>, LinkError> {
        let ports: HashSet<String> = graph
            .list_ports()
            .into_iter()
            .map(|p| format!("{}:{}", p.node, p.port))
            .collect();

        let desired_set: HashSet<(String, String)> = desired.iter().cloned().collect();

        // Disconnect removed
        let to_remove: Vec<_> = self
            .applied
            .iter()
            .filter(|e| !desired_set.contains(e))
            .cloned()
            .collect();
        for (from, to) in to_remove {
            graph.disconnect(&from, &to)?;
            self.applied.remove(&(from, to));
        }

        let mut status = Vec::new();
        for (from, to) in desired {
            let ready = ports.contains(from) && ports.contains(to);
            if ready {
                if !self.applied.contains(&(from.clone(), to.clone())) {
                    graph.connect(from, to)?;
                    self.applied.insert((from.clone(), to.clone()));
                }
                status.push(LinkStatus {
                    from: from.clone(),
                    to: to.clone(),
                    state: LinkState::Applied,
                });
            } else {
                status.push(LinkStatus {
                    from: from.clone(),
                    to: to.clone(),
                    state: LinkState::Pending,
                });
            }
        }
        Ok(status)
    }

    /// Clear applied tracking (after stop).
    pub fn clear(&mut self) {
        self.applied.clear();
    }
}

/// Host `pw-link` graph operations.
#[derive(Debug, Default)]
pub struct PwLinkGraph;

impl PwLinkGraph {
    /// Create a pw-link backend.
    pub fn new() -> Self {
        Self
    }
}

impl GraphOps for PwLinkGraph {
    fn list_ports(&self) -> Vec<GraphPort> {
        let output = Command::new("pw-link").arg("-io").output();
        let Ok(output) = output else {
            return Vec::new();
        };
        let text = String::from_utf8_lossy(&output.stdout);
        let mut ports = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if let Some((node, port)) = line.split_once(':') {
                ports.push(GraphPort {
                    node: node.to_string(),
                    port: port.to_string(),
                    direction: PortDirection::Unknown,
                    card: None,
                });
            }
        }
        ports
    }

    fn connect(&self, from: &str, to: &str) -> Result<(), LinkError> {
        let status = Command::new("pw-link")
            .arg(from)
            .arg(to)
            .status()
            .map_err(|e| LinkError::Message(e.to_string()))?;
        if !status.success() {
            return Err(LinkError::Message(format!(
                "pw-link connect failed: {status}"
            )));
        }
        Ok(())
    }

    fn disconnect(&self, from: &str, to: &str) -> Result<(), LinkError> {
        let status = Command::new("pw-link")
            .arg("-d")
            .arg(from)
            .arg(to)
            .status()
            .map_err(|e| LinkError::Message(e.to_string()))?;
        if !status.success() {
            return Err(LinkError::Message(format!(
                "pw-link disconnect failed: {status}"
            )));
        }
        Ok(())
    }

    fn resolve_card(&self, card: &str) -> Option<String> {
        // Best-effort: look for ALSA nodes whose name or description mentions the card.
        let ports = self.list_ports();
        let mut nodes: HashMap<String, ()> = HashMap::new();
        for p in ports {
            if p.card.as_deref() == Some(card) {
                return Some(p.node);
            }
            nodes.insert(p.node, ());
        }
        // Fallback: node name contains the card id.
        nodes
            .into_keys()
            .find(|n| n.contains(card))
    }
}
