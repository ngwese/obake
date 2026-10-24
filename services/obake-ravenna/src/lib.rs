// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! RAVENNA control plane for an existing PipeWire graph.
//!
//! Pure stream rules, SDP, and qualify logic live here so they can be tested
//! without linking PipeWire or talking to Avahi.

pub mod avahi;
pub mod config;
pub mod destination;
pub mod frame_size;
pub mod pipewire_nodes;
pub mod ptp;
pub mod qualify;
pub mod reconcile;
pub mod rtsp;
pub mod sdp;
pub mod status;
pub mod system_pw;

/// Re-export the on-disk config types most callers need.
pub use config::{Config, Format, SinkConfig, SourceConfig};
/// Re-export stream qualify types used by the binary and status formatting.
pub use qualify::{DownReason, QualifyInput, QualifiedSink, QualifiedSource, StreamState};
