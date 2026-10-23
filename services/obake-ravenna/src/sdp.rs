// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Build and parse RAVENNA/AES67-style SDP for RTP streams.

use crate::config::Format;
use crate::frame_size;
use crate::qualify::QualifiedSource;
use std::net::Ipv4Addr;
use thiserror::Error;

/// Media parameters taken from (or written into) an SDP body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamSdp {
    /// Multicast connection address (`c=`).
    pub group: Ipv4Addr,
    /// RTP port (`m=`).
    pub port: u16,
    /// Sample rate from `rtpmap`.
    pub rate: u32,
    /// Channel count from `rtpmap`.
    pub channels: u32,
    /// L16 or L24 from `rtpmap`.
    pub format: Format,
    /// Samples per packet (`a=framecount` or derived from `a=ptime`).
    pub frame_size: u32,
    /// PTP domain from `a=ts-refclk`.
    pub domain: u8,
}

/// Failure parsing an SDP body.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SdpError {
    /// No usable `c=` connection line.
    #[error("missing connection address")]
    MissingConnection,
    /// No usable `m=` media port.
    #[error("missing media port")]
    MissingPort,
    /// No usable `a=rtpmap` line.
    #[error("missing rtpmap")]
    MissingRtpmap,
    /// Neither `a=framecount` nor a derivable `a=ptime`.
    #[error("missing frame size")]
    MissingFrameSize,
    /// No usable `a=ts-refclk` PTP domain.
    #[error("missing PTP domain")]
    MissingDomain,
    /// Malformed field value or unsupported encoding.
    #[error("invalid SDP: {0}")]
    Invalid(String),
}

/// Build the SDP body for a transmitting source.
pub fn build_source_sdp(src: &QualifiedSource, session_name: &str) -> String {
    let ptime = frame_size::ptime_ms(src.frame_size, src.rate).unwrap_or(1.0);
    let encoding = src.format.as_str();
    format!(
        "v=0\r\n\
o=- 0 0 IN IP4 {group}\r\n\
s={session}\r\n\
c=IN IP4 {group}\r\n\
t=0 0\r\n\
m=audio {port} RTP/AVP 97\r\n\
a=rtpmap:97 {encoding}/{rate}/{channels}\r\n\
a=ptime:{ptime}\r\n\
a=framecount:{frame}\r\n\
a=ts-refclk:ptp=IEEE1588-2008:00-00-00-00-00-00-00-00:{domain}\r\n\
a=mediaclk:direct=0\r\n",
        group = src.destination.group,
        session = session_name,
        port = src.destination.port,
        encoding = encoding,
        rate = src.rate,
        channels = src.channels,
        ptime = format_ptime(ptime),
        frame = src.frame_size,
        domain = src.domain,
    )
}

/// Format `a=ptime` without trailing zeros when possible.
fn format_ptime(ptime: f64) -> String {
    if (ptime - ptime.round()).abs() < 1e-9 {
        format!("{}", ptime.round() as u32)
    } else {
        format!("{ptime:.3}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

/// Parse connection, media, rtpmap, ptime/framecount, and ts-refclk from SDP.
pub fn parse_sdp(text: &str) -> Result<StreamSdp, SdpError> {
    let mut group = None;
    let mut port = None;
    let mut rate = None;
    let mut channels = None;
    let mut format = None;
    let mut frame_size = None;
    let mut ptime = None;
    let mut domain = None;

    for raw in text.lines() {
        let line = raw.trim().trim_end_matches('\r');
        if let Some(rest) = line.strip_prefix("c=IN IP4 ") {
            let addr = rest
                .split('/')
                .next()
                .unwrap_or(rest)
                .parse()
                .map_err(|_| SdpError::Invalid(line.to_string()))?;
            group = Some(addr);
        } else if let Some(rest) = line.strip_prefix("m=audio ") {
            let p: u16 = rest
                .split_whitespace()
                .next()
                .ok_or(SdpError::MissingPort)?
                .parse()
                .map_err(|_| SdpError::Invalid(line.to_string()))?;
            port = Some(p);
        } else if let Some(rest) = line.strip_prefix("a=rtpmap:") {
            // 97 L24/48000/8
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() < 2 {
                return Err(SdpError::Invalid(line.to_string()));
            }
            let map: Vec<&str> = parts[1].split('/').collect();
            if map.len() < 3 {
                return Err(SdpError::Invalid(line.to_string()));
            }
            format = Some(match map[0] {
                "L16" => Format::L16,
                "L24" => Format::L24,
                other => return Err(SdpError::Invalid(other.to_string())),
            });
            rate = Some(
                map[1]
                    .parse()
                    .map_err(|_| SdpError::Invalid(line.to_string()))?,
            );
            channels = Some(
                map[2]
                    .parse()
                    .map_err(|_| SdpError::Invalid(line.to_string()))?,
            );
        } else if let Some(rest) = line.strip_prefix("a=ptime:") {
            ptime = Some(
                rest.parse::<f64>()
                    .map_err(|_| SdpError::Invalid(line.to_string()))?,
            );
        } else if let Some(rest) = line.strip_prefix("a=framecount:") {
            frame_size = Some(
                rest.parse()
                    .map_err(|_| SdpError::Invalid(line.to_string()))?,
            );
        } else if let Some(rest) = line.strip_prefix("a=ts-refclk:ptp=") {
            // IEEE1588-2008:mac:domain
            let domain_str = rest.rsplit(':').next().unwrap_or("");
            domain = Some(
                domain_str
                    .parse()
                    .map_err(|_| SdpError::Invalid(line.to_string()))?,
            );
        }
    }

    let group = group.ok_or(SdpError::MissingConnection)?;
    let port = port.ok_or(SdpError::MissingPort)?;
    let rate = rate.ok_or(SdpError::MissingRtpmap)?;
    let channels = channels.ok_or(SdpError::MissingRtpmap)?;
    let format = format.ok_or(SdpError::MissingRtpmap)?;
    let domain = domain.ok_or(SdpError::MissingDomain)?;

    let frame_size = match (frame_size, ptime) {
        (Some(f), _) => f,
        (None, Some(p)) => ((p * f64::from(rate)) / 1000.0).round() as u32,
        (None, None) => return Err(SdpError::MissingFrameSize),
    };

    Ok(StreamSdp {
        group,
        port,
        rate,
        channels,
        format,
        frame_size,
        domain,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::destination::MulticastDestination;
    use crate::qualify::QualifiedSource;

    fn sample_source() -> QualifiedSource {
        QualifiedSource {
            name: "obake-out-1".into(),
            channels: 8,
            format: Format::L24,
            frame_size: 48,
            destination: MulticastDestination::parse("239.69.1.10:5004").unwrap(),
            ttl: 15,
            rate: 48_000,
            domain: 0,
        }
    }

    #[test]
    fn round_trip_source_sdp() {
        let src = sample_source();
        let text = build_source_sdp(&src, "obake-out-1");
        let parsed = parse_sdp(&text).unwrap();
        assert_eq!(parsed.group, Ipv4Addr::new(239, 69, 1, 10));
        assert_eq!(parsed.port, 5004);
        assert_eq!(parsed.rate, 48_000);
        assert_eq!(parsed.channels, 8);
        assert_eq!(parsed.format, Format::L24);
        assert_eq!(parsed.frame_size, 48);
        assert_eq!(parsed.domain, 0);
        assert!(text.contains("a=ts-refclk:ptp=IEEE1588-2008:"));
        assert!(text.contains(":0\r\n") || text.ends_with(":0\r\n") || text.contains(":0\n"));
    }
}
