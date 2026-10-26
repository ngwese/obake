// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Compiled shape profiles and port naming.

use crate::config::RoleDirection;
use std::path::Path;

/// Shape the service knows about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShapeProfile {
    /// Shape name (`chuck`, `rnbo-runner`, …).
    pub name: &'static str,
    /// Whether this change can launch the shape.
    pub launchable: bool,
}

/// Profiles compiled into the service.
pub const PROFILES: &[ShapeProfile] = &[
    ShapeProfile {
        name: "chuck",
        launchable: true,
    },
    ShapeProfile {
        name: "rnbo-runner",
        launchable: true,
    },
];

/// Look up a compiled profile by name.
pub fn profile(name: &str) -> Option<&'static ShapeProfile> {
    PROFILES.iter().find(|p| p.name == name)
}

/// Whether a shape may be launched when its image is present.
pub fn is_launchable(name: &str) -> bool {
    profile(name).map(|p| p.launchable).unwrap_or(false)
}

/// Map a role slot to the shape's JACK/PipeWire port name.
pub fn port_name(shape: &str, direction: RoleDirection, index: u32) -> Option<String> {
    match shape {
        "chuck" => match direction {
            RoleDirection::Out => Some(format!("outport {index}")),
            RoleDirection::In => Some(format!("inport {index}")),
        },
        "rnbo-runner" => match direction {
            RoleDirection::Out => Some(format!("out{index}")),
            RoleDirection::In => Some(format!("in{index}")),
        },
        _ => None,
    }
}

/// Full `node:port` endpoint for a role.
pub fn endpoint(node: &str, shape: &str, direction: RoleDirection, index: u32) -> Option<String> {
    port_name(shape, direction, index).map(|port| format!("{node}:{port}"))
}

/// Whether a SIF exists for the shape under `image_dir`.
pub fn image_present(image_dir: &Path, shape: &str) -> bool {
    image_dir.join(format!("{shape}.sif")).is_file()
}

/// GraphQL-facing shape listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapeInfo {
    /// Shape name.
    pub name: String,
    /// Launchable when true and image is present.
    pub launchable: bool,
    /// Whether the SIF file exists.
    pub image_present: bool,
}

/// List compiled profiles with image presence.
pub fn list_shapes(image_dir: &Path) -> Vec<ShapeInfo> {
    PROFILES
        .iter()
        .map(|p| {
            let present = image_present(image_dir, p.name);
            ShapeInfo {
                name: p.name.into(),
                launchable: p.launchable && present,
                image_present: present,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chuck_ports() {
        assert_eq!(
            port_name("chuck", RoleDirection::Out, 1).unwrap(),
            "outport 1"
        );
        assert_eq!(
            port_name("chuck", RoleDirection::In, 2).unwrap(),
            "inport 2"
        );
    }

    #[test]
    fn rnbo_ports() {
        assert_eq!(
            port_name("rnbo-runner", RoleDirection::Out, 1).unwrap(),
            "out1"
        );
        assert_eq!(
            port_name("rnbo-runner", RoleDirection::In, 1).unwrap(),
            "in1"
        );
    }
}
