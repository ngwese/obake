// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Desired PipeWire object arguments and a recording client for tests.
//!
//! Production loads modules on the existing daemon via [`crate::system_pw`].

use crate::frame_size;
use crate::qualify::{QualifiedSink, QualifiedSource};
use crate::sdp;
use log::{debug, info};
use std::collections::HashMap;

/// Fallback `sess.ts-refclk` when no grandmaster identity is available.
pub const PTP_CLOCK_TOKEN: &str = "ptp=traceable";

/// Which PipeWire RTP factory to load for a stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleKind {
    /// Local audio → multicast RTP (`libpipewire-module-rtp-sink`).
    RtpSink,
    /// Multicast RTP → local audio (`libpipewire-module-rtp-source`).
    RtpSource,
}

/// Arguments for the PTP graph driver (`support.node.driver`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriverArgs {
    /// `node.name`.
    pub name: String,
    /// `node.group` shared with RTP nodes.
    pub node_group: String,
    /// `clock.interface` (ptp4l netdev).
    pub clock_interface: String,
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
    /// Samples per RTP packet (`rtp.framecount`).
    pub framecount: u32,
    /// Packet time in ms for `sess.latency.msec` (integer multiple of ptime).
    pub sess_latency_msec: String,
    /// `sess.min-ptime` / `sess.max-ptime` (ms, same when locked).
    pub ptime_ms: String,
    /// Multicast group address string.
    pub destination: String,
    pub port: u16,
    /// IP TTL for transmitters; unused for receivers.
    pub ttl: Option<u8>,
    /// Netdev used for multicast join/send.
    pub iface: String,
    /// Value for `sess.ts-refclk`.
    pub ts_refclk: String,
    /// `sess.ts-offset` (epoch frame alignment for AES67 transmitters).
    pub ts_offset: i32,
    /// `net.loop` — true while probing own RTP timestamps for alignment.
    pub net_loop: bool,
    /// RTP dynamic payload type.
    pub payload_type: u8,
    /// `node.group`.
    pub node_group: String,
}

impl ModuleArgs {
    /// Build load args for a transmitting (qualified) source.
    pub fn from_source(src: &QualifiedSource, iface: &str) -> Self {
        let ptime = frame_size::ptime_ms(src.frame_size, src.rate).unwrap_or(1.0);
        let latency = (ptime * 3.0).max(ptime).round().max(1.0);
        Self {
            name: src.name.clone(),
            kind: ModuleKind::RtpSink,
            rate: src.rate,
            channels: src.channels,
            audio_format: src.format.pipewire_audio_format().to_string(),
            framecount: src.frame_size,
            sess_latency_msec: format_latency(latency),
            ptime_ms: format_latency(ptime),
            destination: src.destination.group.to_string(),
            port: src.destination.port,
            ttl: Some(src.ttl),
            iface: iface.to_string(),
            ts_refclk: format!(
                "ptp=IEEE1588-2008:{}:{}",
                sdp::format_gmid(&src.gmid),
                src.domain
            ),
            ts_offset: 0,
            net_loop: false,
            payload_type: src.payload_type,
            node_group: src.node_group.clone(),
        }
    }

    /// Build load args for a receiving (qualified) sink from its SDP.
    pub fn from_sink(sink: &QualifiedSink, iface: &str) -> Self {
        let ptime = frame_size::ptime_ms(sink.sdp.frame_size, sink.sdp.rate).unwrap_or(1.0);
        let latency = (ptime * 3.0).max(ptime).round().max(1.0);
        Self {
            name: sink.name.clone(),
            kind: ModuleKind::RtpSource,
            rate: sink.sdp.rate,
            channels: sink.sdp.channels,
            audio_format: sink.sdp.format.pipewire_audio_format().to_string(),
            framecount: sink.sdp.frame_size,
            sess_latency_msec: format_latency(latency),
            ptime_ms: format_latency(ptime),
            destination: sink.sdp.group.to_string(),
            port: sink.sdp.port,
            ttl: None,
            iface: iface.to_string(),
            ts_refclk: PTP_CLOCK_TOKEN.to_string(),
            ts_offset: 0,
            net_loop: false,
            payload_type: sink.sdp.payload_type,
            node_group: sink.node_group.clone(),
        }
    }

    /// SPA-JSON-ish debug dump of properties applied to PipeWire.
    pub fn debug_props(&self) -> String {
        format!(
            "kind={:?} name={} rate={} channels={} format={} framecount={} \
             ptime={} latency={} dest={}:{} ttl={:?} iface={} ts-refclk={} \
             ts-offset={} payload={} group={}",
            self.kind,
            self.name,
            self.rate,
            self.channels,
            self.audio_format,
            self.framecount,
            self.ptime_ms,
            self.sess_latency_msec,
            self.destination,
            self.port,
            self.ttl,
            self.iface,
            self.ts_refclk,
            self.ts_offset,
            self.payload_type,
            self.node_group,
        )
    }
}

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
    /// Ensure the PTP graph driver exists (create or replace if args change).
    fn ensure_driver(&mut self, args: &DriverArgs) -> Result<(), String>;
    /// Remove the PTP driver if the daemon created one.
    fn remove_driver(&mut self) -> Result<(), String>;
    /// Load (or replace) an RTP module for `args.name`.
    fn load_module(&mut self, args: &ModuleArgs) -> Result<(), String>;
    /// Unload the module previously loaded under `name`.
    fn unload_module(&mut self, name: &str) -> Result<(), String>;
    /// Configured media rate used for qualify and SDP (from daemon config).
    fn graph_rate(&self) -> u32;
}

/// Test / interim client that records load/unload without linking libpipewire.
#[derive(Debug, Default)]
pub struct RecordingPipewire {
    /// Fixed rate returned by [`PipewireClient::graph_rate`].
    pub rate: u32,
    /// Currently loaded modules keyed by stream name.
    pub loaded: HashMap<String, ModuleArgs>,
    /// Active driver, if any.
    pub driver: Option<DriverArgs>,
    /// Ordered mutation history for assertions.
    pub events: Vec<PwEvent>,
}

/// One recorded mutation by [`RecordingPipewire`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PwEvent {
    /// Driver created or replaced.
    EnsureDriver(DriverArgs),
    /// Driver removed.
    RemoveDriver,
    /// RTP module loaded or replaced.
    Load(ModuleArgs),
    /// RTP module unloaded.
    Unload(String),
}

impl RecordingPipewire {
    /// Create a recorder with a fixed graph rate.
    pub fn new(rate: u32) -> Self {
        Self {
            rate,
            loaded: HashMap::new(),
            driver: None,
            events: Vec::new(),
        }
    }
}

impl PipewireClient for RecordingPipewire {
    fn ensure_driver(&mut self, args: &DriverArgs) -> Result<(), String> {
        let action = if self.driver.as_ref() == Some(args) {
            "update"
        } else if self.driver.is_some() {
            "update"
        } else {
            "create"
        };
        info!("pipewire: {action} driver name={}", args.name);
        debug!(
            "pipewire: driver props name={} group={} clock.interface={}",
            args.name, args.node_group, args.clock_interface
        );
        self.driver = Some(args.clone());
        self.events.push(PwEvent::EnsureDriver(args.clone()));
        Ok(())
    }

    fn remove_driver(&mut self) -> Result<(), String> {
        if let Some(d) = self.driver.take() {
            info!("pipewire: delete driver name={}", d.name);
            self.events.push(PwEvent::RemoveDriver);
        }
        Ok(())
    }

    fn load_module(&mut self, args: &ModuleArgs) -> Result<(), String> {
        let action = if self.loaded.contains_key(&args.name) {
            "update"
        } else {
            "create"
        };
        info!("pipewire: {action} stream name={}", args.name);
        debug!("pipewire: stream props {}", args.debug_props());
        self.loaded.insert(args.name.clone(), args.clone());
        self.events.push(PwEvent::Load(args.clone()));
        Ok(())
    }

    fn unload_module(&mut self, name: &str) -> Result<(), String> {
        if self.loaded.remove(name).is_some() {
            info!("pipewire: delete stream name={name}");
            self.events.push(PwEvent::Unload(name.to_string()));
        }
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
    use crate::qualify::QualifiedSource;

    fn sample_source() -> QualifiedSource {
        QualifiedSource {
            name: "obake-out-1".into(),
            channels: 8,
            format: Format::L24,
            frame_size: 48,
            destination: MulticastDestination::parse("239.1.60.50:5004").unwrap(),
            ttl: 15,
            rate: 48_000,
            domain: 0,
            gmid: [0x00, 0x1d, 0xc1, 0xff, 0xfe, 0x12, 0x34, 0x56],
            payload_type: 104,
            node_group: "obake.ravenna".into(),
            source_ip: std::net::Ipv4Addr::new(192, 168, 60, 50),
        }
    }

    #[test]
    fn from_source_sets_payload_and_framecount() {
        let args = ModuleArgs::from_source(&sample_source(), "enp3s0");
        assert_eq!(args.payload_type, 104);
        assert_eq!(args.framecount, 48);
        assert_eq!(
            args.ts_refclk,
            "ptp=IEEE1588-2008:00-1D-C1-FF-FE-12-34-56:0"
        );
        assert_eq!(args.node_group, "obake.ravenna");
        assert!(!args.debug_props().contains("aes67.driver-group"));
    }

    #[test]
    fn recording_driver_and_stream_lifecycle() {
        let mut pw = RecordingPipewire::new(48_000);
        let driver = DriverArgs {
            name: "obake-ptp-driver".into(),
            node_group: "obake.ravenna".into(),
            clock_interface: "enp3s0".into(),
        };
        pw.ensure_driver(&driver).unwrap();
        pw.ensure_driver(&driver).unwrap();
        assert_eq!(pw.driver.as_ref(), Some(&driver));
        let args = ModuleArgs::from_source(&sample_source(), "enp3s0");
        assert!(args.debug_props().contains("payload=104"));
        assert!(args.debug_props().contains("framecount=48"));
        pw.load_module(&args).unwrap();
        pw.unload_module("obake-out-1").unwrap();
        pw.remove_driver().unwrap();
        assert!(pw.driver.is_none());
        assert!(pw.loaded.is_empty());
        assert_eq!(
            pw.events
                .iter()
                .filter(|e| matches!(e, PwEvent::EnsureDriver(_)))
                .count(),
            2
        );
        assert!(pw.events.iter().any(|e| matches!(e, PwEvent::Load(_))));
        assert!(pw.events.iter().any(|e| matches!(e, PwEvent::Unload(_))));
        assert!(pw.events.iter().any(|e| matches!(e, PwEvent::RemoveDriver)));
    }
}
