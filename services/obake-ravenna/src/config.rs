// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! On-disk stream configuration (`config.toml`) for sources and sinks.

use serde::Deserialize;
use std::fs;
use std::path::Path;
use thiserror::Error;

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
    /// Samples per RTP packet (RAVENNA "frame size").
    pub frame_size: u32,
    /// Multicast `group` or `group:port` destination.
    pub destination: String,
    /// Optional IP TTL; defaults to 15 when omitted.
    #[serde(default)]
    pub ttl: Option<u8>,
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

/// Full daemon configuration: zero or more sources and sinks.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
pub struct Config {
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
}

impl Config {
    /// Parse configuration from a TOML string.
    pub fn from_str(text: &str) -> Result<Self, ConfigError> {
        Ok(toml::from_str(text)?)
    }

    /// Read and parse a config file from disk.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        Self::from_str(&fs::read_to_string(path)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = r#"
[[source]]
name = "obake-out-1"
channels = 8
format = "L24"
frame_size = 48
destination = "239.69.1.10:5004"
ttl = 15

[[sink]]
name = "obake-in-1"
device = "Studio-IO"
stream = "PGM-1"
"#;

    #[test]
    fn parses_design_example() {
        let cfg = Config::from_str(EXAMPLE).unwrap();
        assert_eq!(cfg.source.len(), 1);
        assert_eq!(cfg.sink.len(), 1);
        let src = &cfg.source[0];
        assert_eq!(src.name, "obake-out-1");
        assert_eq!(src.channels, 8);
        assert_eq!(src.format, Format::L24);
        assert_eq!(src.frame_size, 48);
        assert_eq!(src.destination, "239.69.1.10:5004");
        assert_eq!(src.ttl, Some(15));
        let sink = &cfg.sink[0];
        assert_eq!(sink.name, "obake-in-1");
        assert_eq!(sink.device, "Studio-IO");
        assert_eq!(sink.stream, "PGM-1");
    }

    #[test]
    fn empty_config_has_no_streams() {
        let cfg = Config::from_str("").unwrap();
        assert!(cfg.source.is_empty());
        assert!(cfg.sink.is_empty());
    }

    #[test]
    fn rejects_unknown_format() {
        let text = r#"
[[source]]
name = "x"
channels = 2
format = "L32"
frame_size = 48
destination = "239.69.1.10:5004"
"#;
        let err = Config::from_str(text).unwrap_err();
        assert!(matches!(err, ConfigError::Parse(_)));
    }

    #[test]
    fn ttl_defaults_to_none() {
        let text = r#"
[[source]]
name = "x"
channels = 2
format = "L16"
frame_size = 48
destination = "239.69.1.10"
"#;
        let cfg = Config::from_str(text).unwrap();
        assert_eq!(cfg.source[0].ttl, None);
    }
}
