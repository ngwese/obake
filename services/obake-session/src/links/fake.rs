// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! In-memory PipeWire graph for tests.

use super::{GraphOps, LinkError};
use std::collections::HashSet;
use std::sync::Mutex;

/// Port direction for listing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortDirection {
    /// Input.
    In,
    /// Output.
    Out,
    /// Unknown (from pw-link -io).
    Unknown,
}

/// One port on the fake graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphPort {
    /// Node name.
    pub node: String,
    /// Port name.
    pub port: String,
    /// Direction.
    pub direction: PortDirection,
    /// Optional ALSA card id for alias resolution.
    pub card: Option<String>,
}

/// Fake graph that records connect/disconnect.
#[derive(Debug, Default)]
pub struct FakeGraph {
    /// Visible ports.
    pub ports: Mutex<Vec<GraphPort>>,
    /// Applied links.
    pub links: Mutex<HashSet<(String, String)>>,
}

impl FakeGraph {
    /// Create an empty fake graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace visible ports.
    pub fn set_ports(&self, ports: Vec<GraphPort>) {
        *self.ports.lock().unwrap() = ports;
    }

    /// Add a port.
    pub fn add_port(
        &self,
        node: &str,
        port: &str,
        direction: PortDirection,
        card: Option<&str>,
    ) {
        self.ports.lock().unwrap().push(GraphPort {
            node: node.into(),
            port: port.into(),
            direction,
            card: card.map(str::to_string),
        });
    }

    /// Snapshot of applied links.
    pub fn links(&self) -> HashSet<(String, String)> {
        self.links.lock().unwrap().clone()
    }
}

impl GraphOps for FakeGraph {
    fn list_ports(&self) -> Vec<GraphPort> {
        self.ports.lock().unwrap().clone()
    }

    fn connect(&self, from: &str, to: &str) -> Result<(), LinkError> {
        self.links
            .lock()
            .unwrap()
            .insert((from.into(), to.into()));
        Ok(())
    }

    fn disconnect(&self, from: &str, to: &str) -> Result<(), LinkError> {
        self.links
            .lock()
            .unwrap()
            .remove(&(from.into(), to.into()));
        Ok(())
    }

    fn resolve_card(&self, card: &str) -> Option<String> {
        self.ports
            .lock()
            .unwrap()
            .iter()
            .find(|p| p.card.as_deref() == Some(card))
            .map(|p| p.node.clone())
    }
}
