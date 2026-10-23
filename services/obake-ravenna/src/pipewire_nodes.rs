// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Desired PipeWire RTP module arguments and a fake client for tests.
//!
//! The real process loads `libpipewire-module-rtp-sink` /
//! `libpipewire-module-rtp-source` on the existing daemon. Unit tests record
//! load/unload by configured name without linking libpipewire.

use crate::frame_size;
use crate::qualify::{QualifiedSink, QualifiedSource};
use std::collections::HashMap;

/// PipeWire 1.6.9 RTP modules take `sess.ts-refclk` as a clock token.
/// When a free-form IEEE1588 string is rejected, pass this token and keep the
/// domain in the served SDP instead.
pub const PTP_CLOCK_TOKEN: &str = "ptp";

/// Which PipeWire RTP factory to load for a stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleKind {
    /// Local audio → multicast RTP (`libpipewire-module-rtp-sink`).
    RtpSink,
    /// Multicast RTP → local audio (`libpipewire-module-rtp-source`).
    RtpSource,
}

/// Arguments needed to load one RTP module instance for a stream name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleArgs {
    /// Configured stream name (also the module instance key).
    pub name: String,
    pub kind: ModuleKind,
    pub rate: u32,
    pub channels: u32,
    /// PipeWire `audio.format` (e.g. `S24BE`).
    pub audio_format: String,
    /// Packet time in ms for `sess.latency.msec`.
    pub sess_latency_msec: String,
    /// Multicast group address string.
    pub destination: String,
    pub port: u16,
    /// IP TTL for transmitters; unused for receivers.
    pub ttl: Option<u8>,
    /// Netdev used for multicast join/send.
    pub iface: String,
    /// Value for `sess.ts-refclk` (see [`PTP_CLOCK_TOKEN`]).
    pub ts_refclk: String,
}

impl ModuleArgs {
    /// Build load args for a transmitting (qualified) source.
    pub fn from_source(src: &QualifiedSource, iface: &str) -> Self {
        let latency = frame_size::ptime_ms(src.frame_size, src.rate).unwrap_or(1.0);
        Self {
            name: src.name.clone(),
            kind: ModuleKind::RtpSink,
            rate: src.rate,
            channels: src.channels,
            audio_format: src.format.pipewire_audio_format().to_string(),
            sess_latency_msec: format_latency(latency),
            destination: src.destination.group.to_string(),
            port: src.destination.port,
            ttl: Some(src.ttl),
            iface: iface.to_string(),
            ts_refclk: PTP_CLOCK_TOKEN.to_string(),
        }
    }

    /// Build load args for a receiving (qualified) sink from its SDP.
    pub fn from_sink(sink: &QualifiedSink, iface: &str) -> Self {
        let latency = frame_size::ptime_ms(sink.sdp.frame_size, sink.sdp.rate).unwrap_or(1.0);
        Self {
            name: sink.name.clone(),
            kind: ModuleKind::RtpSource,
            rate: sink.sdp.rate,
            channels: sink.sdp.channels,
            audio_format: sink.sdp.format.pipewire_audio_format().to_string(),
            sess_latency_msec: format_latency(latency),
            destination: sink.sdp.group.to_string(),
            port: sink.sdp.port,
            ttl: None,
            iface: iface.to_string(),
            ts_refclk: PTP_CLOCK_TOKEN.to_string(),
        }
    }
}

/// Format `sess.latency.msec` without needless trailing zeros.
fn format_latency(ms: f64) -> String {
    if (ms - ms.round()).abs() < 1e-9 {
        format!("{}", ms.round() as u32)
    } else {
        format!("{ms:.3}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

/// Abstraction over the PipeWire daemon used by reconcile and the main loop.
pub trait PipewireClient {
    /// Load (or replace) an RTP module for `args.name`.
    fn load_module(&mut self, args: &ModuleArgs) -> Result<(), String>;
    /// Unload the module previously loaded under `name`.
    fn unload_module(&mut self, name: &str) -> Result<(), String>;
    /// Live graph `clock.rate` used for qualify and SDP.
    fn graph_rate(&self) -> u32;
}

/// Test / interim client that records load/unload without linking libpipewire.
#[derive(Debug, Default)]
pub struct RecordingPipewire {
    /// Fixed graph rate returned by [`PipewireClient::graph_rate`].
    pub rate: u32,
    /// Currently “loaded” modules keyed by stream name.
    pub loaded: HashMap<String, ModuleArgs>,
    /// Ordered load/unload history for assertions.
    pub events: Vec<PwEvent>,
}

/// One recorded mutation by [`RecordingPipewire`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PwEvent {
    Load(String),
    Unload(String),
}

impl RecordingPipewire {
    /// Create a recording client reporting a fixed sample rate.
    pub fn new(rate: u32) -> Self {
        Self {
            rate,
            loaded: HashMap::new(),
            events: Vec::new(),
        }
    }
}

impl PipewireClient for RecordingPipewire {
    fn load_module(&mut self, args: &ModuleArgs) -> Result<(), String> {
        self.loaded.insert(args.name.clone(), args.clone());
        self.events.push(PwEvent::Load(args.name.clone()));
        Ok(())
    }

    fn unload_module(&mut self, name: &str) -> Result<(), String> {
        self.loaded.remove(name);
        self.events.push(PwEvent::Unload(name.to_string()));
        Ok(())
    }

    fn graph_rate(&self) -> u32 {
        self.rate
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Format;
    use crate::destination::MulticastDestination;
    use crate::qualify::{QualifiedSink, QualifiedSource};
    use crate::sdp::StreamSdp;
    use std::net::Ipv4Addr;

    #[test]
    fn source_maps_l24_and_frame_size() {
        let src = QualifiedSource {
            name: "out".into(),
            channels: 8,
            format: Format::L24,
            frame_size: 48,
            destination: MulticastDestination::parse("239.69.1.10:5004").unwrap(),
            ttl: 15,
            rate: 48_000,
            domain: 0,
        };
        let args = ModuleArgs::from_source(&src, "enp1s0");
        assert_eq!(args.kind, ModuleKind::RtpSink);
        assert_eq!(args.audio_format, "S24BE");
        assert_eq!(args.sess_latency_msec, "1");
        assert_eq!(args.destination, "239.69.1.10");
        assert_eq!(args.port, 5004);
        assert_eq!(args.ttl, Some(15));
        assert_eq!(args.iface, "enp1s0");
        assert_eq!(args.ts_refclk, PTP_CLOCK_TOKEN);
    }

    #[test]
    fn sink_maps_from_sdp() {
        let sink = QualifiedSink {
            name: "in".into(),
            device: "Studio-IO".into(),
            stream: "PGM-1".into(),
            sdp: StreamSdp {
                group: Ipv4Addr::new(239, 69, 1, 20),
                port: 5004,
                rate: 48_000,
                channels: 2,
                format: Format::L16,
                frame_size: 48,
                domain: 0,
            },
            rate: 48_000,
        };
        let args = ModuleArgs::from_sink(&sink, "enp1s0");
        assert_eq!(args.kind, ModuleKind::RtpSource);
        assert_eq!(args.audio_format, "S16BE");
    }
}
