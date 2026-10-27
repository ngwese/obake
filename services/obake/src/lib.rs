// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! GraphQL client library for `obake-session`.

mod client;
mod config;
pub mod format;
pub mod xfer;

pub use client::{Client, ClientError, Endpoint};
pub use config::{
    default_config_path, load_config, resolve_endpoint, ClientConfig, HostConfig, ResolveError,
    Resolved,
};
pub use format::Kind;
