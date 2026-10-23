// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Pure stream up/down decisions from config, PTP lock, and discovered SDP.

use crate::config::{Config, Format, SinkConfig, SourceConfig};
use crate::destination::{resolve_ttl, MulticastDestination};
use crate::frame_size;
use crate::sdp::StreamSdp;
use std::collections::HashMap;
use std::net::Ipv4Addr;

/// Why a configured stream is down (shown by `status`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownReason {
    /// `ptp4l` is inactive or not in SLAVE.
    Unlocked,
    /// Graph sample rate is not an AES67 / RAVENNA media clock rate.
    BadSampleRate,
    /// Frame size is not an allowed packet time at the graph rate.
    BadFrameSize,
    /// Destination failed multicast / port rules.
    BadDestination,
    /// Another local source uses the same group and port.
    DuplicateDestination,
    /// `ptp4l` interface is missing or still a placeholder.
    InterfaceMissing,
    /// Sink's device/stream was not discovered.
    DeviceAbsent,
    /// Remote SDP connection address is not scoped multicast.
    UnicastSdp,
    /// Remote SDP sample rate differs from the live graph.
    RateMismatch,
    /// Remote SDP PTP domain differs from local `ptp4l`.
    DomainMismatch,
}

impl DownReason {
    /// Stable kebab-case token for status output and logs.
    pub fn as_str(self) -> &'static str {
        match self {
            DownReason::Unlocked => "unlocked",
            DownReason::BadSampleRate => "bad-sample-rate",
            DownReason::BadFrameSize => "bad-frame-size",
            DownReason::BadDestination => "bad-destination",
            DownReason::DuplicateDestination => "duplicate-destination",
            DownReason::InterfaceMissing => "interface-missing",
            DownReason::DeviceAbsent => "device-absent",
            DownReason::UnicastSdp => "unicast-sdp",
            DownReason::RateMismatch => "rate-mismatch",
            DownReason::DomainMismatch => "domain-mismatch",
        }
    }
}

/// A source that passed all checks and may load an RTP sink module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualifiedSource {
    /// Configured local name.
    pub name: String,
    pub channels: u32,
    pub format: Format,
    pub frame_size: u32,
    pub destination: MulticastDestination,
    pub ttl: u8,
    /// Live PipeWire graph sample rate (not from config).
    pub rate: u32,
    /// Local `ptp4l` domain for SDP `ts-refclk`.
    pub domain: u8,
}

/// A sink that passed all checks and may load an RTP source module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualifiedSink {
    /// Configured local PipeWire node name.
    pub name: String,
    /// Remote DNS-SD device instance name.
    pub device: String,
    /// Remote stream name on that device.
    pub stream: String,
    /// Remote stream description used for the join.
    pub sdp: StreamSdp,
    /// Live graph rate (must equal `sdp.rate`).
    pub rate: u32,
}

/// Per-stream result of [`qualify`]: transmit/receive or held down.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamState {
    /// Source is eligible to transmit.
    SourceUp(QualifiedSource),
    /// Source is configured but held down.
    SourceDown { name: String, reason: DownReason },
    /// Sink is eligible to receive.
    SinkUp(QualifiedSink),
    /// Sink is configured but held down.
    SinkDown { name: String, reason: DownReason },
}

impl StreamState {
    /// Configured stream name.
    pub fn name(&self) -> &str {
        match self {
            StreamState::SourceUp(s) => &s.name,
            StreamState::SourceDown { name, .. } => name,
            StreamState::SinkUp(s) => &s.name,
            StreamState::SinkDown { name, .. } => name,
        }
    }

    /// `"source"` or `"sink"`.
    pub fn kind(&self) -> &'static str {
        match self {
            StreamState::SourceUp(_) | StreamState::SourceDown { .. } => "source",
            StreamState::SinkUp(_) | StreamState::SinkDown { .. } => "sink",
        }
    }

    /// True when the stream may have an active PipeWire RTP node.
    pub fn is_up(&self) -> bool {
        matches!(
            self,
            StreamState::SourceUp(_) | StreamState::SinkUp(_)
        )
    }

    /// Hold-down reason when down.
    pub fn down_reason(&self) -> Option<DownReason> {
        match self {
            StreamState::SourceDown { reason, .. } | StreamState::SinkDown { reason, .. } => {
                Some(*reason)
            }
            _ => None,
        }
    }

    /// Compact status string for logs (`source foo up`, `sink bar down:unlocked`).
    pub fn status_label(&self) -> String {
        match self {
            StreamState::SourceUp(s) => format!("source {} up", s.name),
            StreamState::SourceDown { name, reason } => {
                format!("source {name} down:{}", reason.as_str())
            }
            StreamState::SinkUp(s) => format!("sink {} up", s.name),
            StreamState::SinkDown { name, reason } => {
                format!("sink {name} down:{}", reason.as_str())
            }
        }
    }
}

/// Inputs to the pure qualify step for one daemon tick.
#[derive(Debug, Clone)]
pub struct QualifyInput<'a> {
    /// Loaded on-disk configuration.
    pub config: &'a Config,
    /// PipeWire graph `clock.rate`.
    pub graph_rate: u32,
    /// True when `ptp4l` port state is SLAVE.
    pub locked: bool,
    /// Local PTP domain number.
    pub domain: u8,
    /// True when the `ptp4l` interface name exists as a netdev.
    pub interface_present: bool,
    /// Map of `(device, stream)` → SDP from DNS-SD / DESCRIBE.
    pub discovered: &'a HashMap<(String, String), StreamSdp>,
}

/// Decide up/down for every configured source and sink.
pub fn qualify(input: &QualifyInput<'_>) -> Vec<StreamState> {
    let mut out = Vec::new();

    let mut dest_owners: HashMap<(Ipv4Addr, u16), Vec<usize>> = HashMap::new();
    let mut parsed: Vec<Option<(SourceConfig, MulticastDestination, u8)>> =
        Vec::with_capacity(input.config.source.len());

    for (idx, src) in input.config.source.iter().enumerate() {
        match MulticastDestination::parse(&src.destination) {
            Ok(dest) => {
                let ttl = resolve_ttl(src.ttl);
                dest_owners
                    .entry((dest.group, dest.port))
                    .or_default()
                    .push(idx);
                parsed.push(Some((src.clone(), dest, ttl)));
            }
            Err(_) => parsed.push(None),
        }
    }

    let duplicates: std::collections::HashSet<usize> = dest_owners
        .values()
        .filter(|owners| owners.len() > 1)
        .flat_map(|owners| owners.iter().copied())
        .collect();

    for (idx, src) in input.config.source.iter().enumerate() {
        out.push(qualify_source(input, src, idx, &parsed, &duplicates));
    }
    for sink in &input.config.sink {
        out.push(qualify_sink(input, sink));
    }
    out
}

/// Qualify a single source against lock, interface, frame size, and destination.
fn qualify_source(
    input: &QualifyInput<'_>,
    src: &SourceConfig,
    idx: usize,
    parsed: &[Option<(SourceConfig, MulticastDestination, u8)>],
    duplicates: &std::collections::HashSet<usize>,
) -> StreamState {
    let down = |reason| StreamState::SourceDown {
        name: src.name.clone(),
        reason,
    };

    if !input.locked {
        return down(DownReason::Unlocked);
    }
    if !input.interface_present {
        return down(DownReason::InterfaceMissing);
    }
    if !frame_size::is_legal_rate(input.graph_rate) {
        return down(DownReason::BadSampleRate);
    }
    if !frame_size::is_legal(src.frame_size, input.graph_rate) {
        return down(DownReason::BadFrameSize);
    }
    let Some((_, dest, ttl)) = parsed[idx].clone() else {
        return down(DownReason::BadDestination);
    };
    if duplicates.contains(&idx) {
        return down(DownReason::DuplicateDestination);
    }

    StreamState::SourceUp(QualifiedSource {
        name: src.name.clone(),
        channels: src.channels,
        format: src.format.clone(),
        frame_size: src.frame_size,
        destination: dest,
        ttl,
        rate: input.graph_rate,
        domain: input.domain,
    })
}

/// Qualify a single sink against lock, discovery, and SDP constraints.
fn qualify_sink(input: &QualifyInput<'_>, sink: &SinkConfig) -> StreamState {
    let down = |reason| StreamState::SinkDown {
        name: sink.name.clone(),
        reason,
    };

    if !input.locked {
        return down(DownReason::Unlocked);
    }
    if !input.interface_present {
        return down(DownReason::InterfaceMissing);
    }
    if !frame_size::is_legal_rate(input.graph_rate) {
        return down(DownReason::BadSampleRate);
    }

    let key = (sink.device.clone(), sink.stream.clone());
    let Some(sdp) = input.discovered.get(&key) else {
        return down(DownReason::DeviceAbsent);
    };
    if !sdp.group.is_multicast() || sdp.group.octets()[0] != 239 {
        return down(DownReason::UnicastSdp);
    }
    if sdp.rate != input.graph_rate {
        return down(DownReason::RateMismatch);
    }
    if sdp.domain != input.domain {
        return down(DownReason::DomainMismatch);
    }

    StreamState::SinkUp(QualifiedSink {
        name: sink.name.clone(),
        device: sink.device.clone(),
        stream: sink.stream.clone(),
        sdp: sdp.clone(),
        rate: input.graph_rate,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Format;
    use std::net::Ipv4Addr;

    fn base_config() -> Config {
        Config {
            source: vec![SourceConfig {
                name: "obake-out-1".into(),
                channels: 8,
                format: Format::L24,
                frame_size: 48,
                destination: "239.69.1.10:5004".into(),
                ttl: None,
            }],
            sink: vec![SinkConfig {
                name: "obake-in-1".into(),
                device: "Studio-IO".into(),
                stream: "PGM-1".into(),
            }],
        }
    }

    fn good_sdp() -> StreamSdp {
        StreamSdp {
            group: Ipv4Addr::new(239, 69, 1, 20),
            port: 5004,
            rate: 48_000,
            channels: 2,
            format: Format::L24,
            frame_size: 48,
            domain: 0,
        }
    }

    #[test]
    fn legal_source_and_sink_go_up() {
        let cfg = base_config();
        let mut discovered = HashMap::new();
        discovered.insert(("Studio-IO".into(), "PGM-1".into()), good_sdp());
        let states = qualify(&QualifyInput {
            config: &cfg,
            graph_rate: 48_000,
            locked: true,
            domain: 0,
            interface_present: true,
            discovered: &discovered,
        });
        assert!(matches!(&states[0], StreamState::SourceUp(_)));
        assert!(matches!(&states[1], StreamState::SinkUp(_)));
    }

    #[test]
    fn sixty_four_samples_go_up() {
        let mut cfg = base_config();
        cfg.source[0].frame_size = 64;
        let discovered = HashMap::new();
        let states = qualify(&QualifyInput {
            config: &cfg,
            graph_rate: 48_000,
            locked: true,
            domain: 0,
            interface_present: true,
            discovered: &discovered,
        });
        assert!(matches!(&states[0], StreamState::SourceUp(_)));
    }

    #[test]
    fn forty_samples_held_down() {
        let mut cfg = base_config();
        cfg.source[0].frame_size = 40;
        let discovered = HashMap::new();
        let states = qualify(&QualifyInput {
            config: &cfg,
            graph_rate: 48_000,
            locked: true,
            domain: 0,
            interface_present: true,
            discovered: &discovered,
        });
        assert_eq!(
            states[0],
            StreamState::SourceDown {
                name: "obake-out-1".into(),
                reason: DownReason::BadFrameSize,
            }
        );
    }

    #[test]
    fn unlocked_holds_source() {
        let cfg = base_config();
        let discovered = HashMap::new();
        let states = qualify(&QualifyInput {
            config: &cfg,
            graph_rate: 48_000,
            locked: false,
            domain: 0,
            interface_present: true,
            discovered: &discovered,
        });
        assert_eq!(
            states[0],
            StreamState::SourceDown {
                name: "obake-out-1".into(),
                reason: DownReason::Unlocked,
            }
        );
    }

    #[test]
    fn unicast_sdp_holds_sink() {
        let cfg = base_config();
        let mut sdp = good_sdp();
        sdp.group = Ipv4Addr::new(192, 168, 1, 10);
        let mut discovered = HashMap::new();
        discovered.insert(("Studio-IO".into(), "PGM-1".into()), sdp);
        let states = qualify(&QualifyInput {
            config: &cfg,
            graph_rate: 48_000,
            locked: true,
            domain: 0,
            interface_present: true,
            discovered: &discovered,
        });
        assert_eq!(
            states[1],
            StreamState::SinkDown {
                name: "obake-in-1".into(),
                reason: DownReason::UnicastSdp,
            }
        );
    }

    #[test]
    fn rate_mismatch_holds_sink() {
        let cfg = base_config();
        let mut sdp = good_sdp();
        sdp.rate = 96_000;
        let mut discovered = HashMap::new();
        discovered.insert(("Studio-IO".into(), "PGM-1".into()), sdp);
        let states = qualify(&QualifyInput {
            config: &cfg,
            graph_rate: 48_000,
            locked: true,
            domain: 0,
            interface_present: true,
            discovered: &discovered,
        });
        assert_eq!(
            states[1],
            StreamState::SinkDown {
                name: "obake-in-1".into(),
                reason: DownReason::RateMismatch,
            }
        );
    }

    #[test]
    fn domain_mismatch_holds_sink() {
        let cfg = base_config();
        let mut sdp = good_sdp();
        sdp.domain = 1;
        let mut discovered = HashMap::new();
        discovered.insert(("Studio-IO".into(), "PGM-1".into()), sdp);
        let states = qualify(&QualifyInput {
            config: &cfg,
            graph_rate: 48_000,
            locked: true,
            domain: 0,
            interface_present: true,
            discovered: &discovered,
        });
        assert_eq!(
            states[1],
            StreamState::SinkDown {
                name: "obake-in-1".into(),
                reason: DownReason::DomainMismatch,
            }
        );
    }

    #[test]
    fn duplicate_destinations_hold_both() {
        let cfg = Config {
            source: vec![
                SourceConfig {
                    name: "a".into(),
                    channels: 2,
                    format: Format::L16,
                    frame_size: 48,
                    destination: "239.69.1.10:5004".into(),
                    ttl: None,
                },
                SourceConfig {
                    name: "b".into(),
                    channels: 2,
                    format: Format::L16,
                    frame_size: 48,
                    destination: "239.69.1.10:5004".into(),
                    ttl: None,
                },
            ],
            sink: vec![],
        };
        let discovered = HashMap::new();
        let states = qualify(&QualifyInput {
            config: &cfg,
            graph_rate: 48_000,
            locked: true,
            domain: 0,
            interface_present: true,
            discovered: &discovered,
        });
        assert!(matches!(
            states[0],
            StreamState::SourceDown {
                reason: DownReason::DuplicateDestination,
                ..
            }
        ));
        assert!(matches!(
            states[1],
            StreamState::SourceDown {
                reason: DownReason::DuplicateDestination,
                ..
            }
        ));
    }

    #[test]
    fn qualify_uses_supplied_graph_rate() {
        let mut cfg = base_config();
        // 96 samples is 1 ms at 96 kHz.
        cfg.source[0].frame_size = 96;
        let discovered = HashMap::new();
        let states = qualify(&QualifyInput {
            config: &cfg,
            graph_rate: 96_000,
            locked: true,
            domain: 0,
            interface_present: true,
            discovered: &discovered,
        });
        match &states[0] {
            StreamState::SourceUp(s) => assert_eq!(s.rate, 96_000),
            other => panic!("expected up, got {other:?}"),
        }
    }

    #[test]
    fn forty_four_one_with_nominal_frame_goes_up() {
        let mut cfg = base_config();
        cfg.source[0].frame_size = 48;
        let discovered = HashMap::new();
        let states = qualify(&QualifyInput {
            config: &cfg,
            graph_rate: 44_100,
            locked: true,
            domain: 0,
            interface_present: true,
            discovered: &discovered,
        });
        match &states[0] {
            StreamState::SourceUp(s) => assert_eq!(s.rate, 44_100),
            other => panic!("expected up, got {other:?}"),
        }
    }

    #[test]
    fn unsupported_graph_rate_holds_source() {
        let mut cfg = base_config();
        cfg.source[0].frame_size = 32;
        let discovered = HashMap::new();
        let states = qualify(&QualifyInput {
            config: &cfg,
            graph_rate: 32_000,
            locked: true,
            domain: 0,
            interface_present: true,
            discovered: &discovered,
        });
        assert_eq!(
            states[0],
            StreamState::SourceDown {
                name: "obake-out-1".into(),
                reason: DownReason::BadSampleRate,
            }
        );
    }
}
