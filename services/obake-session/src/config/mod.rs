// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Host profile and rig document parsing.

mod rig;
mod host;

pub use rig::{
    load_rig, load_rigs, parse_rig, Rig, RigError, RigLink,
    Instance, RoleDirection, RoleRef,
};
pub use host::{
    load_host, parse_host, Alias, HostBind, HostProfile, HostProfileError, SampleLibrary,
};
