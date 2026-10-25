// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! On-disk daemon configuration (`config.toml`) for graph, packet, and streams.

use serde::Deserialize;
use std::fs;
use std::net::Ipv4Addr;
use std::path::Path;
use thiserror::Error;

use crate::destination::{
    resolve_ttl, DestinationError, MulticastDestination, DEFAULT_RTP_PORT, DEFAULT_TTL,
};

/// Linear PCM sample format advertised in SDP (`L16` / `L24`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Format {
    /// 16-bit big-endian PCM.
    L16,
    /// 24-bit big-endian PCM.
    L24,
}

impl Format {
    /// SDP `rtpmap` encoding name.
    pub fn as_str(&self) -> &'static str {
        match self {
            Format::L16 => "L16",
            Format::L24 => "L24",
        }
    }

    /// PipeWire `audio.format` argument for the RTP modules.
    pub fn pipewire_audio_format(&self) -> &'static str {
        match self {
            Format::L16 => "S16BE",
            Format::L24 => "S24BE",
        }
    }

    /// Bytes per sample on the wire (L24 is 3).
    pub fn bytes_per_sample(&self) -> u32 {
        match self {
            Format::L16 => 2,
            Format::L24 => 3,
        }
    }
}

/// Shared media clock for every RAVENNA object the daemon creates.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GraphConfig {
    /// Sample rate in Hz (AES67 / RAVENNA media clock).
    pub rate: u32,
    /// PipeWire `node.group` for the PTP driver and RTP nodes.
    #[serde(default = "default_node_group")]
    pub node_group: String,
}

fn default_node_group() -> String {
    "obake.ravenna".into()
}

impl Default for GraphConfig {
    fn default() -> Self {
        Self {
            rate: 48_000,
            node_group: default_node_group(),
        }
    }
}

/// RTP packetization (independent of the PipeWire quantum).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PacketConfig {
    /// Samples per RTP packet (`a=framecount` / `rtp.framecount`).
    pub frames: u32,
}

impl Default for PacketConfig {
    fn default() -> Self {
        Self { frames: 48 }
    }
}

/// Transmit multicast base address and port stepping.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct MulticastConfig {
    /// Shared multicast group for transmitting sources.
    pub address: String,
    /// Even base RTP port; source `i` uses `base_port + 2 * i`.
    #[serde(default = "default_base_port")]
    pub base_port: u16,
    /// IP TTL for transmitters; defaults to 15 when omitted.
    #[serde(default)]
    pub ttl: Option<u8>,
}

fn default_base_port() -> u16 {
    DEFAULT_RTP_PORT
}

/// One configured multicast transmitter (local → network).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SourceConfig {
    /// Local PipeWire node / DESCRIBE stream name.
    pub name: String,
    /// Channel count in the RTP payload.
    pub channels: u32,
    /// Payload sample format.
    pub format: Format,
    /// Optional RTP dynamic payload type (default `96 + channels`).
    #[serde(default)]
    pub payload: Option<u8>,
    /// Optional pinned multicast address (else `[multicast].address`).
    #[serde(default)]
    pub address: Option<String>,
    /// Optional pinned even RTP port (else `base_port + 2 * index`).
    #[serde(default)]
    pub port: Option<u16>,
}

/// One configured multicast receiver (network → local).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SinkConfig {
    /// Local PipeWire node name.
    pub name: String,
    /// Remote DNS-SD instance name to browse for.
    pub device: String,
    /// Stream name on that device (DESCRIBE `/by-name/<stream>`).
    pub stream: String,
}

/// Default continuous PTP lock dwell before media nodes may come up (ms).
pub const DEFAULT_LOCK_HOLD_MS: u64 = 2000;

fn default_lock_hold_ms() -> u64 {
    DEFAULT_LOCK_HOLD_MS
}

/// PTP-related daemon settings (not the `ptp4l` config file).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PtpConfig {
    /// Require `ptp4l` FOLLOWER continuously for this many milliseconds before
    /// sources/sinks qualify. `0` brings media up on the lock edge.
    #[serde(default = "default_lock_hold_ms")]
    pub lock_hold_ms: u64,
}

impl Default for PtpConfig {
    fn default() -> Self {
        Self {
            lock_hold_ms: DEFAULT_LOCK_HOLD_MS,
        }
    }
}

/// Full daemon configuration.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
pub struct Config {
    /// Sample rate and node group.
    #[serde(default)]
    pub graph: GraphConfig,
    /// RTP frames per packet.
    #[serde(default)]
    pub packet: PacketConfig,
    /// Lock-hold dwell and related PTP gating.
    #[serde(default)]
    pub ptp: PtpConfig,
    /// Transmit multicast base (required when any `[[source]]` is present).
    #[serde(default)]
    pub multicast: Option<MulticastConfig>,
    /// Transmitters defined in `[[source]]` tables.
    #[serde(default)]
    pub source: Vec<SourceConfig>,
    /// Receivers defined in `[[sink]]` tables.
    #[serde(default)]
    pub sink: Vec<SinkConfig>,
}

/// Failure loading or parsing the TOML config file.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Filesystem error reading the config path.
    #[error("read config: {0}")]
    Io(#[from] std::io::Error),
    /// TOML deserialize error.
    #[error("parse config: {0}")]
    Parse(#[from] toml::de::Error),
    /// Semantic validation failed after parse.
    #[error("{0}")]
    Invalid(String),
}

impl Config {
    /// Parse configuration from a TOML string and validate semantics.
    pub fn from_str(text: &str) -> Result<Self, ConfigError> {
        let cfg: Config = toml::from_str(text)?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Read and parse a config file from disk.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        Self::from_str(&fs::read_to_string(path)?)
    }

    /// Require `[multicast]` when any source is configured; check base port.
    fn validate(&self) -> Result<(), ConfigError> {
        if !self.source.is_empty() {
            let Some(mc) = &self.multicast else {
                return Err(ConfigError::Invalid(
                    "missing [multicast] address (required when [[source]] is present)".into(),
                ));
            };
            if mc.base_port % 2 != 0 {
                return Err(ConfigError::Invalid(format!(
                    "multicast.base_port {} must be even",
                    mc.base_port
                )));
            }
            MulticastDestination::new(
                mc.address
                    .parse::<Ipv4Addr>()
                    .map_err(|e| ConfigError::Invalid(format!("multicast.address: {e}")))?,
                mc.base_port,
            )
            .map_err(|e| ConfigError::Invalid(format!("multicast.address: {e}")))?;
        }
        Ok(())
    }

    /// Effective TTL for transmitters.
    pub fn multicast_ttl(&self) -> u8 {
        self.multicast
            .as_ref()
            .map(|m| resolve_ttl(m.ttl))
            .unwrap_or(DEFAULT_TTL)
    }

    /// Resolve the multicast destination for source at `index`.
    pub fn resolve_source_destination(
        &self,
        index: usize,
        src: &SourceConfig,
    ) -> Result<MulticastDestination, DestinationError> {
        let mc = self.multicast.as_ref().ok_or_else(|| {
            DestinationError::Parse("missing [multicast]".into())
        })?;
        let group: Ipv4Addr = match &src.address {
            Some(a) => a.parse().map_err(|_| DestinationError::Parse(a.clone()))?,
            None => mc
                .address
                .parse()
                .map_err(|_| DestinationError::Parse(mc.address.clone()))?,
        };
        let port = match src.port {
            Some(p) => p,
            None => {
                let step = u16::try_from(index)
                    .ok()
                    .and_then(|i| i.checked_mul(2))
                    .ok_or_else(|| DestinationError::Parse("source index overflow".into()))?;
                mc.base_port
                    .checked_add(step)
                    .ok_or_else(|| DestinationError::Parse("port overflow".into()))?
            }
        };
        MulticastDestination::new(group, port)
    }

    /// Resolve RTP payload type (`96 + channels` or override in 96..=127).
    pub fn resolve_payload(src: &SourceConfig) -> Result<u8, DestinationError> {
        let pt = match src.payload {
            Some(p) => p,
            None => {
                let ch = u8::try_from(src.channels)
                    .map_err(|_| DestinationError::Parse("channels overflow".into()))?;
                96u8.checked_add(ch).ok_or_else(|| {
                    DestinationError::Parse("payload type overflow".into())
                })?
            }
        };
        if !(96..=127).contains(&pt) {
            return Err(DestinationError::Parse(format!(
                "payload type {pt} out of range 96..=127"
            )));
        }
        Ok(pt)
    }
}

/// RAVENNA conservative audio payload limit on a 1500-byte path (bytes).
pub const MAX_AUDIO_PAYLOAD: u32 = 1440;

/// Audio payload bytes for one RTP packet.
pub fn audio_payload_bytes(channels: u32, frames: u32, format: &Format) -> u32 {
    channels
        .saturating_mul(frames)
        .saturating_mul(format.bytes_per_sample())
}

/// True when one packet of this size fits without fragmentation.
pub fn payload_fits_mtu(channels: u32, frames: u32, format: &Format) -> bool {
    audio_payload_bytes(channels, frames, format) <= MAX_AUDIO_PAYLOAD
}

#[cfg(test)]
mod tests {
    use super::*;

    const TYPICAL: &str = r#"
[graph]
rate = 48000
node_group = "obake.ravenna"

[packet]
frames = 48

[multicast]
address = "239.1.60.50"
base_port = 5004
ttl = 15

[[source]]
name = "obake-out-1"
channels = 8
format = "L24"

[[sink]]
name = "obake-in-1"
device = "Anubis"
stream = "Anubis_Combo 1/2"
"#;

    #[test]
    fn parses_typical_example() {
        let cfg = Config::from_str(TYPICAL).unwrap();
        assert_eq!(cfg.graph.rate, 48_000);
        assert_eq!(cfg.graph.node_group, "obake.ravenna");
        assert_eq!(cfg.packet.frames, 48);
        assert_eq!(cfg.ptp.lock_hold_ms, DEFAULT_LOCK_HOLD_MS);
        let mc = cfg.multicast.as_ref().unwrap();
        assert_eq!(mc.address, "239.1.60.50");
        assert_eq!(mc.base_port, 5004);
        assert_eq!(mc.ttl, Some(15));
        assert_eq!(cfg.source.len(), 1);
        assert_eq!(cfg.sink.len(), 1);
        let dest = cfg
            .resolve_source_destination(0, &cfg.source[0])
            .unwrap();
        assert_eq!(dest.group, Ipv4Addr::new(239, 1, 60, 50));
        assert_eq!(dest.port, 5004);
        assert_eq!(Config::resolve_payload(&cfg.source[0]).unwrap(), 104);
    }

    #[test]
    fn empty_config_has_no_streams() {
        let cfg = Config::from_str("").unwrap();
        assert!(cfg.source.is_empty());
        assert!(cfg.sink.is_empty());
        assert!(cfg.multicast.is_none());
        assert_eq!(cfg.ptp.lock_hold_ms, DEFAULT_LOCK_HOLD_MS);
    }

    #[test]
    fn absent_ptp_table_defaults_lock_hold() {
        let cfg = Config::from_str("[graph]\nrate = 48000\n").unwrap();
        assert_eq!(cfg.ptp.lock_hold_ms, 2000);
    }

    #[test]
    fn explicit_zero_lock_hold() {
        let cfg = Config::from_str("[ptp]\nlock_hold_ms = 0\n").unwrap();
        assert_eq!(cfg.ptp.lock_hold_ms, 0);
    }

    #[test]
    fn explicit_lock_hold_value() {
        let cfg = Config::from_str("[ptp]\nlock_hold_ms = 5000\n").unwrap();
        assert_eq!(cfg.ptp.lock_hold_ms, 5000);
    }

    #[test]
    fn rejects_source_without_multicast() {
        let text = r#"
[[source]]
name = "x"
channels = 2
format = "L16"
"#;
        let err = Config::from_str(text).unwrap_err();
        assert!(matches!(err, ConfigError::Invalid(_)));
    }

    #[test]
    fn rejects_odd_base_port() {
        let text = r#"
[multicast]
address = "239.1.60.50"
base_port = 5005

[[source]]
name = "x"
channels = 2
format = "L16"
"#;
        assert!(Config::from_str(text).is_err());
    }

    #[test]
    fn rejects_sap_group() {
        let text = r#"
[multicast]
address = "239.255.255.255"
base_port = 5004

[[source]]
name = "x"
channels = 2
format = "L16"
"#;
        assert!(Config::from_str(text).is_err());
    }

    #[test]
    fn stepped_ports() {
        let text = r#"
[multicast]
address = "239.1.60.50"
base_port = 5004

[[source]]
name = "a"
channels = 2
format = "L16"

[[source]]
name = "b"
channels = 2
format = "L16"
"#;
        let cfg = Config::from_str(text).unwrap();
        assert_eq!(
            cfg.resolve_source_destination(0, &cfg.source[0])
                .unwrap()
                .port,
            5004
        );
        assert_eq!(
            cfg.resolve_source_destination(1, &cfg.source[1])
                .unwrap()
                .port,
            5006
        );
    }

    #[test]
    fn pin_address_and_port() {
        let text = r#"
[multicast]
address = "239.1.60.50"
base_port = 5004

[[source]]
name = "a"
channels = 2
format = "L16"
address = "239.1.60.51"
port = 5010
"#;
        let cfg = Config::from_str(text).unwrap();
        let dest = cfg
            .resolve_source_destination(0, &cfg.source[0])
            .unwrap();
        assert_eq!(dest.group, Ipv4Addr::new(239, 1, 60, 51));
        assert_eq!(dest.port, 5010);
    }

    #[test]
    fn payload_defaults_and_override() {
        let text = r#"
[multicast]
address = "239.1.60.50"

[[source]]
name = "a"
channels = 2
format = "L16"

[[source]]
name = "b"
channels = 8
format = "L24"
payload = 110
"#;
        let cfg = Config::from_str(text).unwrap();
        assert_eq!(Config::resolve_payload(&cfg.source[0]).unwrap(), 98);
        assert_eq!(Config::resolve_payload(&cfg.source[1]).unwrap(), 110);
    }

    #[test]
    fn payload_out_of_range() {
        let src = SourceConfig {
            name: "x".into(),
            channels: 2,
            format: Format::L16,
            payload: Some(95),
            address: None,
            port: None,
        };
        assert!(Config::resolve_payload(&src).is_err());
    }

    #[test]
    fn mtu_gate() {
        assert!(payload_fits_mtu(8, 48, &Format::L24));
        assert!(!payload_fits_mtu(8, 64, &Format::L24));
        assert_eq!(audio_payload_bytes(8, 48, &Format::L24), 1152);
        assert_eq!(audio_payload_bytes(8, 64, &Format::L24), 1536);
    }

    #[test]
    fn ttl_defaults_to_15() {
        let text = r#"
[multicast]
address = "239.1.60.50"

[[source]]
name = "x"
channels = 2
format = "L16"
"#;
        let cfg = Config::from_str(text).unwrap();
        assert_eq!(cfg.multicast_ttl(), 15);
    }

    #[test]
    fn parses_examples_config_toml() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/config.toml");
        let cfg = Config::load(&path).unwrap();
        assert_eq!(cfg.graph.rate, 48_000);
        assert_eq!(cfg.graph.node_group, "obake.ravenna");
        assert_eq!(cfg.packet.frames, 48);
        assert_eq!(cfg.source.len(), 1);
        assert_eq!(cfg.sink.len(), 1);
        let dest = cfg
            .resolve_source_destination(0, &cfg.source[0])
            .unwrap();
        assert_eq!(dest.group, Ipv4Addr::new(239, 1, 60, 50));
        assert_eq!(dest.port, 5004);
    }

    #[test]
    fn image_seed_defines_no_streams() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../image/os/buildroot/package/obake-ravenna/config.toml");
        if !path.exists() {
            return;
        }
        let cfg = Config::load(&path).unwrap();
        assert!(cfg.source.is_empty());
        assert!(cfg.sink.is_empty());
    }
}
