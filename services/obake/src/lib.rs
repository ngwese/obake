// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! GraphQL client library for `obake-session`.

pub mod auth;
mod client;
mod config;
pub mod device;
pub mod format;
pub mod rig;
pub mod xfer;

pub use client::{Client, ClientError, Endpoint};
pub use config::{
    default_config_path, load_config, missing_remote_setup, resolve_endpoint, runtime_base,
    runtime_from, save_config,
    ClientConfig, HostConfig, ResolveError, Resolved,
};
pub use format::Kind;
