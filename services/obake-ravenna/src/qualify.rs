// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Pure stream up/down decisions from config, PTP lock, and discovered SDP.

use crate::config::{
    payload_fits_mtu, Config, Format, SinkConfig, SourceConfig,
};
use crate::destination::MulticastDestination;
use crate::frame_size;
use crate::sdp::StreamSdp;
use std::collections::HashMap;
use std::net::Ipv4Addr;

/// Why a configured stream is down (shown by `status`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownReason {
    /// `ptp4l` is inactive or not in FOLLOWER.
    Unlocked,
    /// Locked, but the configured continuous lock-hold has not elapsed.
    LockHold,
    /// Locked and hold elapsed, but absolute PTP offset exceeds the threshold
    /// (or the offset could not be read while the threshold is non-zero).
    OffsetHigh,
    /// No grandmaster clock identity from `ptp4l`.
    NoGrandmaster,
    /// Configured sample rate is not an AES67 / RAVENNA media clock rate.
    BadSampleRate,
    /// Frames per packet is not an allowed packet time at the rate.
    BadFrameSize,
    /// Destination failed multicast / port rules.
    BadDestination,
    /// Another local source uses the same group and port.
    DuplicateDestination,
    /// Audio payload exceeds 1440 bytes (would need fragmentation).
    PayloadTooLarge,
    /// Payload type outside 96..=127.
    BadPayloadType,
    /// `ptp4l` interface is missing or still a placeholder.
    InterfaceMissing,
    /// Sink's device/stream was not discovered.
    DeviceAbsent,
    /// Remote SDP connection address is not scoped multicast.
    UnicastSdp,
    /// Remote SDP sample rate differs from the configured graph rate.
    RateMismatch,
    /// Remote SDP PTP domain differs from local `ptp4l`.
    DomainMismatch,
}

impl DownReason {
    /// Stable kebab-case token for status output and logs.
    pub fn as_str(self) -> &'static str {
        match self {
            DownReason::Unlocked => "unlocked",
            DownReason::LockHold => "lock-hold",
            DownReason::OffsetHigh => "offset-high",
            DownReason::NoGrandmaster => "no-grandmaster",
            DownReason::BadSampleRate => "bad-sample-rate",
            DownReason::BadFrameSize => "bad-frame-size",
            DownReason::BadDestination => "bad-destination",
            DownReason::DuplicateDestination => "duplicate-destination",
            DownReason::PayloadTooLarge => "payload-too-large",
            DownReason::BadPayloadType => "bad-payload-type",
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
    /// RTP frames per packet (from `[packet].frames`).
    pub frame_size: u32,
    pub destination: MulticastDestination,
    pub ttl: u8,
    /// Configured graph sample rate.
    pub rate: u32,
    /// Local `ptp4l` domain for SDP `ts-refclk`.
    pub domain: u8,
    /// Grandmaster clock identity bytes (8) for SDP (non-zero).
    pub gmid: [u8; 8],
    /// RTP dynamic payload type (96..=127).
    pub payload_type: u8,
    /// PipeWire node group.
    pub node_group: String,
    /// Unicast IPv4 of the PTP/egress interface (`o=` / `source-filter`).
    pub source_ip: Ipv4Addr,
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
    /// Configured graph rate (must equal `sdp.rate`).
    pub rate: u32,
    /// PipeWire node group.
    pub node_group: String,
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
            _ => None
        }
    }

    /// Compact status string for logs.
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
    /// True when `ptp4l` port state is FOLLOWER.
    pub locked: bool,
    /// True when FOLLOWER has been continuous for the configured lock-hold.
    pub lock_hold_satisfied: bool,
    /// Absolute PTP offset from master in nanoseconds, if known.
    pub master_offset_ns: Option<i64>,
    /// Local PTP domain number.
    pub domain: u8,
    /// Grandmaster identity from PARENT_DATA_SET (None if unknown/zero).
    pub gmid: Option<[u8; 8]>,
    /// True when the `ptp4l` interface name exists as a netdev.
    pub interface_present: bool,
    /// Unicast IPv4 of that interface (for SDP `source-filter` / `o=`).
    pub source_ip: Option<Ipv4Addr>,
    /// Map of `(device, stream)` → SDP from DNS-SD / DESCRIBE.
    pub discovered: &'a HashMap<(String, String), StreamSdp>,
}

/// True when the offset gate allows media (threshold `0`, or |offset| ≤ max).
fn offset_within_threshold(offset: Option<i64>, max_offset_ns: u64) -> bool {
    if max_offset_ns == 0 {
        return true;
    }
    match offset {
        Some(o) => o.unsigned_abs() <= max_offset_ns,
        None => false,
    }
}

/// Decide up/down for every configured source and sink.
pub fn qualify(input: &QualifyInput<'_>) -> Vec<StreamState> {
    let mut out = Vec::new();
    let rate = input.config.graph.rate;
    let frames = input.config.packet.frames;
    let ttl = input.config.multicast_ttl();

    let mut dest_owners: HashMap<(Ipv4Addr, u16), Vec<usize>> = HashMap::new();
    let mut parsed: Vec<Option<(MulticastDestination, u8)>> =
        Vec::with_capacity(input.config.source.len());

    for (idx, src) in input.config.source.iter().enumerate() {
        match (
            input.config.resolve_source_destination(idx, src),
            Config::resolve_payload(src),
        ) {
            (Ok(dest), Ok(pt)) => {
                dest_owners
                    .entry((dest.group, dest.port))
                    .or_default()
                    .push(idx);
                parsed.push(Some((dest, pt)));
            }
            (Err(_), _) => parsed.push(None),
            (Ok(_), Err(_)) => parsed.push(None),
        }
    }

    let duplicates: std::collections::HashSet<usize> = dest_owners
        .values()
        .filter(|owners| owners.len() > 1)
        .flat_map(|owners| owners.iter().copied())
        .collect();

    for (idx, src) in input.config.source.iter().enumerate() {
        out.push(qualify_source(
            input, src, idx, rate, frames, ttl, &parsed, &duplicates,
        ));
    }
    for sink in &input.config.sink {
        out.push(qualify_sink(input, sink, rate));
    }
    out
}

fn qualify_source(
    input: &QualifyInput<'_>,
    src: &SourceConfig,
    idx: usize,
    rate: u32,
    frames: u32,
    ttl: u8,
    parsed: &[Option<(MulticastDestination, u8)>],
    duplicates: &std::collections::HashSet<usize>,
) -> StreamState {
    let down = |reason| StreamState::SourceDown {
        name: src.name.clone(),
        reason,
    };

    if !input.locked {
        return down(DownReason::Unlocked);
    }
    if !input.lock_hold_satisfied {
        return down(DownReason::LockHold);
    }
    if !offset_within_threshold(input.master_offset_ns, input.config.ptp.max_offset_ns) {
        return down(DownReason::OffsetHigh);
    }
    let Some(gmid) = input.gmid else {
        return down(DownReason::NoGrandmaster);
    };
    if gmid == [0u8; 8] {
        return down(DownReason::NoGrandmaster);
    }
    if !input.interface_present {
        return down(DownReason::InterfaceMissing);
    }
    let Some(source_ip) = input.source_ip else {
        return down(DownReason::InterfaceMissing);
    };
    if !frame_size::is_legal_rate(rate) {
        return down(DownReason::BadSampleRate);
    }
    if !frame_size::is_legal(frames, rate) {
        return down(DownReason::BadFrameSize);
    }
    if !payload_fits_mtu(src.channels, frames, &src.format) {
        return down(DownReason::PayloadTooLarge);
    }

    let Some((dest, payload_type)) = parsed[idx].clone() else {
        // Distinguish destination vs payload failure.
        if input.config.resolve_source_destination(idx, src).is_err() {
            return down(DownReason::BadDestination);
        }
        return down(DownReason::BadPayloadType);
    };
    if duplicates.contains(&idx) {
        return down(DownReason::DuplicateDestination);
    }

    StreamState::SourceUp(QualifiedSource {
        name: src.name.clone(),
        channels: src.channels,
        format: src.format.clone(),
        frame_size: frames,
        destination: dest,
        ttl,
        rate,
        domain: input.domain,
        gmid,
        payload_type,
        node_group: input.config.graph.node_group.clone(),
        source_ip,
    })
}

fn qualify_sink(input: &QualifyInput<'_>, sink: &SinkConfig, rate: u32) -> StreamState {
    let down = |reason| StreamState::SinkDown {
        name: sink.name.clone(),
        reason,
    };

    if !input.locked {
        return down(DownReason::Unlocked);
    }
    if !input.lock_hold_satisfied {
        return down(DownReason::LockHold);
    }
    if !offset_within_threshold(input.master_offset_ns, input.config.ptp.max_offset_ns) {
        return down(DownReason::OffsetHigh);
    }
    if input.gmid.is_none() || input.gmid == Some([0u8; 8]) {
        return down(DownReason::NoGrandmaster);
    }
    if !input.interface_present {
        return down(DownReason::InterfaceMissing);
    }
    if !frame_size::is_legal_rate(rate) {
        return down(DownReason::BadSampleRate);
    }

    let key = (sink.device.clone(), sink.stream.clone());
    let Some(sdp) = input.discovered.get(&key) else {
        return down(DownReason::DeviceAbsent);
    };
    if !sdp.group.is_multicast() || sdp.group.octets()[0] != 239 {
        return down(DownReason::UnicastSdp);
    }
    if sdp.rate != rate {
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
        rate,
        node_group: input.config.graph.node_group.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{GraphConfig, MulticastConfig, PacketConfig, PtpConfig};

    fn base_config() -> Config {
        Config {
            graph: GraphConfig {
                rate: 48_000,
                node_group: "obake.ravenna".into(),
            },
            packet: PacketConfig { frames: 48 },
            ptp: PtpConfig::default(),
            multicast: Some(MulticastConfig {
                address: "239.1.60.50".into(),
                base_port: 5004,
                ttl: None,
            }),
            source: vec![SourceConfig {
                name: "obake-out-1".into(),
                channels: 8,
                format: Format::L24,
                payload: None,
                address: None,
                port: None,
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
            payload_type: 98,
        }
    }

    fn gmid() -> [u8; 8] {
        [0x00, 0x1d, 0xc1, 0xff, 0xfe, 0x12, 0x34, 0x56]
    }

    fn input<'a>(
        cfg: &'a Config,
        discovered: &'a HashMap<(String, String), StreamSdp>,
    ) -> QualifyInput<'a> {
        QualifyInput {
            config: cfg,
            locked: true,
            lock_hold_satisfied: true,
            master_offset_ns: Some(0),
            domain: 0,
            gmid: Some(gmid()),
            interface_present: true,
            source_ip: Some(Ipv4Addr::new(192, 168, 60, 50)),
            discovered,
        }
    }

    #[test]
    fn lock_hold_not_elapsed_holds_source_and_sink() {
        let cfg = base_config();
        let mut discovered = HashMap::new();
        discovered.insert(("Studio-IO".into(), "PGM-1".into()), good_sdp());
        let mut qi = input(&cfg, &discovered);
        qi.lock_hold_satisfied = false;
        let states = qualify(&qi);
        assert_eq!(states[0].down_reason(), Some(DownReason::LockHold));
        assert_eq!(states[1].down_reason(), Some(DownReason::LockHold));
    }

    #[test]
    fn lock_hold_elapsed_allows_up() {
        let cfg = base_config();
        let mut discovered = HashMap::new();
        discovered.insert(("Studio-IO".into(), "PGM-1".into()), good_sdp());
        let states = qualify(&input(&cfg, &discovered));
        assert!(matches!(&states[0], StreamState::SourceUp(_)));
        assert!(matches!(&states[1], StreamState::SinkUp(_)));
    }

    #[test]
    fn offset_above_threshold_holds_source_and_sink() {
        let cfg = base_config();
        let mut discovered = HashMap::new();
        discovered.insert(("Studio-IO".into(), "PGM-1".into()), good_sdp());
        let mut qi = input(&cfg, &discovered);
        qi.master_offset_ns = Some(2_000_000);
        let states = qualify(&qi);
        assert_eq!(states[0].down_reason(), Some(DownReason::OffsetHigh));
        assert_eq!(states[1].down_reason(), Some(DownReason::OffsetHigh));
    }

    #[test]
    fn offset_within_threshold_allows_up() {
        let cfg = base_config();
        let mut discovered = HashMap::new();
        discovered.insert(("Studio-IO".into(), "PGM-1".into()), good_sdp());
        let mut qi = input(&cfg, &discovered);
        qi.master_offset_ns = Some(500_000);
        let states = qualify(&qi);
        assert!(matches!(&states[0], StreamState::SourceUp(_)));
        assert!(matches!(&states[1], StreamState::SinkUp(_)));
    }

    #[test]
    fn zero_max_offset_disables_gate() {
        let mut cfg = base_config();
        cfg.ptp.max_offset_ns = 0;
        let mut discovered = HashMap::new();
        discovered.insert(("Studio-IO".into(), "PGM-1".into()), good_sdp());
        let mut qi = input(&cfg, &discovered);
        qi.master_offset_ns = Some(i64::MAX);
        let states = qualify(&qi);
        assert!(matches!(&states[0], StreamState::SourceUp(_)));
        assert!(matches!(&states[1], StreamState::SinkUp(_)));
    }

    #[test]
    fn missing_offset_holds_when_threshold_nonzero() {
        let cfg = base_config();
        let mut discovered = HashMap::new();
        discovered.insert(("Studio-IO".into(), "PGM-1".into()), good_sdp());
        let mut qi = input(&cfg, &discovered);
        qi.master_offset_ns = None;
        let states = qualify(&qi);
        assert_eq!(states[0].down_reason(), Some(DownReason::OffsetHigh));
        assert_eq!(states[1].down_reason(), Some(DownReason::OffsetHigh));
    }

    #[test]
    fn legal_source_and_sink_go_up() {
        let cfg = base_config();
        let mut discovered = HashMap::new();
        discovered.insert(("Studio-IO".into(), "PGM-1".into()), good_sdp());
        let states = qualify(&input(&cfg, &discovered));
        match &states[0] {
            StreamState::SourceUp(s) => {
                assert_eq!(s.payload_type, 104);
                assert_eq!(s.destination.port, 5004);
                assert_eq!(s.gmid, gmid());
            }
            other => panic!("expected up, got {other:?}"),
        }
        assert!(matches!(&states[1], StreamState::SinkUp(_)));
    }

    #[test]
    fn sixty_four_frames_eight_ch_l24_held_for_mtu() {
        let mut cfg = base_config();
        cfg.packet.frames = 64;
        let discovered = HashMap::new();
        let states = qualify(&input(&cfg, &discovered));
        assert_eq!(
            states[0],
            StreamState::SourceDown {
                name: "obake-out-1".into(),
                reason: DownReason::PayloadTooLarge,
            }
        );
    }

    #[test]
    fn sixty_four_frames_two_ch_goes_up() {
        let mut cfg = base_config();
        cfg.packet.frames = 64;
        cfg.source[0].channels = 2;
        let discovered = HashMap::new();
        let states = qualify(&input(&cfg, &discovered));
        assert!(matches!(&states[0], StreamState::SourceUp(_)));
    }

    #[test]
    fn forty_eight_frames_qualify_without_quantum_input() {
        // Packet size is independent of the PipeWire quantum: qualify has
        // no quantum field, so 48-frame packets go up regardless of a
        // 128-sample graph cycle on the host.
        let cfg = base_config();
        let discovered = HashMap::new();
        let states = qualify(&input(&cfg, &discovered));
        assert!(matches!(&states[0], StreamState::SourceUp(s) if s.frame_size == 48));
    }

    #[test]
    fn bad_payload_override_holds() {
        let mut cfg = base_config();
        cfg.source[0].payload = Some(95);
        let discovered = HashMap::new();
        let states = qualify(&input(&cfg, &discovered));
        assert_eq!(
            states[0].down_reason(),
            Some(DownReason::BadPayloadType)
        );
    }

    #[test]
    fn missing_gmid_holds() {
        let cfg = base_config();
        let discovered = HashMap::new();
        let mut qi = input(&cfg, &discovered);
        qi.gmid = None;
        let states = qualify(&qi);
        assert_eq!(
            states[0].down_reason(),
            Some(DownReason::NoGrandmaster)
        );
    }

    #[test]
    fn zero_gmid_holds() {
        let cfg = base_config();
        let discovered = HashMap::new();
        let mut qi = input(&cfg, &discovered);
        qi.gmid = Some([0u8; 8]);
        let states = qualify(&qi);
        assert_eq!(
            states[0].down_reason(),
            Some(DownReason::NoGrandmaster)
        );
    }

    #[test]
    fn forty_frames_held_down() {
        let mut cfg = base_config();
        cfg.packet.frames = 40;
        let discovered = HashMap::new();
        let states = qualify(&input(&cfg, &discovered));
        assert_eq!(
            states[0].down_reason(),
            Some(DownReason::BadFrameSize)
        );
    }

    #[test]
    fn stepped_ports_no_duplicate() {
        let mut cfg = base_config();
        cfg.source.push(SourceConfig {
            name: "obake-out-2".into(),
            channels: 2,
            format: Format::L16,
            payload: None,
            address: None,
            port: None,
        });
        let discovered = HashMap::new();
        let states = qualify(&input(&cfg, &discovered));
        match (&states[0], &states[1]) {
            (StreamState::SourceUp(a), StreamState::SourceUp(b)) => {
                assert_eq!(a.destination.port, 5004);
                assert_eq!(b.destination.port, 5006);
            }
            other => panic!("expected both up, got {other:?}"),
        }
    }

    #[test]
    fn duplicate_pinned_ports_hold_both() {
        let mut cfg = base_config();
        cfg.source[0].port = Some(5004);
        cfg.source.push(SourceConfig {
            name: "b".into(),
            channels: 2,
            format: Format::L16,
            payload: None,
            address: None,
            port: Some(5004),
        });
        let discovered = HashMap::new();
        let states = qualify(&input(&cfg, &discovered));
        assert_eq!(
            states[0].down_reason(),
            Some(DownReason::DuplicateDestination)
        );
        assert_eq!(
            states[1].down_reason(),
            Some(DownReason::DuplicateDestination)
        );
    }
}
