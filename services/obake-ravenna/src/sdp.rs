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
    /// Dynamic payload type from `m=` / `a=rtpmap`.
    pub payload_type: u8,
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

/// Format an 8-byte clock identity as uppercase hex with dashes.
pub fn format_gmid(gmid: &[u8; 8]) -> String {
    format!(
        "{:02X}-{:02X}-{:02X}-{:02X}-{:02X}-{:02X}-{:02X}-{:02X}",
        gmid[0], gmid[1], gmid[2], gmid[3], gmid[4], gmid[5], gmid[6], gmid[7]
    )
}

/// Build the SDP body for a transmitting source.
///
/// Shape matches Merging/Anubis session DESCRIBE (clock-domain, TTL on
/// `c=`, source-filter, sync-time/palign, recvonly) so peers fill CLOCK
/// DOMAIN and can join the multicast cleanly.
pub fn build_source_sdp(src: &QualifiedSource, session_name: &str) -> String {
    let ptime = frame_size::ptime_ms(src.frame_size, src.rate).unwrap_or(1.0);
    let encoding = src.format.as_str();
    let gmid = format_gmid(&src.gmid);
    let pt = src.payload_type;
    let group = src.destination.group;
    let ttl = src.ttl;
    let src_ip = src.source_ip;
    let refclk = format!("ptp=IEEE1588-2008:{gmid}:{}", src.domain);
    format!(
        "v=0\r\n\
o=- 0 0 IN IP4 {src_ip}\r\n\
s={session}\r\n\
c=IN IP4 {group}/{ttl}\r\n\
t=0 0\r\n\
a=clock-domain:PTPv2 {domain}\r\n\
a=ts-refclk:{refclk}\r\n\
a=mediaclk:direct=0\r\n\
m=audio {port} RTP/AVP {pt}\r\n\
c=IN IP4 {group}/{ttl}\r\n\
a=rtpmap:{pt} {encoding}/{rate}/{channels}\r\n\
a=source-filter: incl IN IP4 {group} {src_ip}\r\n\
a=clock-domain:PTPv2 {domain}\r\n\
a=sync-time:0\r\n\
a=framecount:{frame}\r\n\
a=palign:0\r\n\
a=ptime:{ptime}\r\n\
a=ts-refclk:{refclk}\r\n\
a=mediaclk:direct=0\r\n\
a=recvonly\r\n",
        session = session_name,
        port = src.destination.port,
        encoding = encoding,
        rate = src.rate,
        channels = src.channels,
        ptime = format_ptime(ptime),
        frame = src.frame_size,
        domain = src.domain,
        pt = pt,
        group = group,
        ttl = ttl,
        src_ip = src_ip,
        refclk = refclk,
    )
}

/// Parse `a=framecount` as a single value or `low-high` range (lower bound).
pub fn parse_framecount(text: &str) -> Result<u32, ()> {
    let lower = text.split('-').next().unwrap_or(text).trim();
    lower.parse().map_err(|_| ())
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
    let mut payload_type = None;

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
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() < 3 {
                return Err(SdpError::MissingPort);
            }
            let p: u16 = parts[0]
                .parse()
                .map_err(|_| SdpError::Invalid(line.to_string()))?;
            port = Some(p);
            if let Ok(pt) = parts[2].parse::<u8>() {
                payload_type = Some(pt);
            }
        } else if let Some(rest) = line.strip_prefix("a=rtpmap:") {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() < 2 {
                return Err(SdpError::Invalid(line.to_string()));
            }
            if payload_type.is_none() {
                if let Ok(pt) = parts[0].parse::<u8>() {
                    payload_type = Some(pt);
                }
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
            // Merging MAD may advertise a range (`48-192`). Use the lower
            // bound unless a caller specifically needs the full range.
            frame_size = Some(parse_framecount(rest).map_err(|_| SdpError::Invalid(line.to_string()))?);
        } else if let Some(rest) = line.strip_prefix("a=ts-refclk:ptp=") {
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
    let payload_type = payload_type.unwrap_or(96);

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
        payload_type,
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
            destination: MulticastDestination::parse("239.1.60.50:5004").unwrap(),
            ttl: 15,
            rate: 48_000,
            domain: 0,
            gmid: [0x00, 0x1d, 0xc1, 0xff, 0xfe, 0x12, 0x34, 0x56],
            payload_type: 104,
            node_group: "obake.ravenna".into(),
            source_ip: Ipv4Addr::new(192, 168, 60, 50),
        }
    }

    #[test]
    fn round_trip_source_sdp() {
        let src = sample_source();
        let text = build_source_sdp(&src, "obake-out-1");
        assert!(text.contains("a=ts-refclk:ptp=IEEE1588-2008:00-1D-C1-FF-FE-12-34-56:0"));
        assert!(!text.contains("00-00-00-00-00-00-00-00"));
        assert!(text.contains("m=audio 5004 RTP/AVP 104"));
        assert!(text.contains("a=rtpmap:104 L24/48000/8"));
        assert!(text.contains("a=clock-domain:PTPv2 0"));
        assert!(text.contains("c=IN IP4 239.1.60.50/15"));
        assert!(text.contains("a=source-filter: incl IN IP4 239.1.60.50 192.168.60.50"));
        assert!(text.contains("a=recvonly"));
        assert!(text.contains("o=- 0 0 IN IP4 192.168.60.50"));
        let parsed = parse_sdp(&text).unwrap();
        assert_eq!(parsed.group, Ipv4Addr::new(239, 1, 60, 50));
        assert_eq!(parsed.port, 5004);
        assert_eq!(parsed.rate, 48_000);
        assert_eq!(parsed.channels, 8);
        assert_eq!(parsed.format, Format::L24);
        assert_eq!(parsed.frame_size, 48);
        assert_eq!(parsed.domain, 0);
        assert_eq!(parsed.payload_type, 104);
    }

    #[test]
    fn rejects_zero_gmid_in_builder_input_via_format() {
        // Builder formats whatever it is given; qualify must refuse zeros.
        let gmid = [0u8; 8];
        assert_eq!(format_gmid(&gmid), "00-00-00-00-00-00-00-00");
    }

    #[test]
    fn parse_framecount_single_and_range() {
        assert_eq!(parse_framecount("48").unwrap(), 48);
        assert_eq!(parse_framecount("48-192").unwrap(), 48);
        assert!(parse_framecount("nope").is_err());
    }

    #[test]
    fn parse_sdp_accepts_merging_framecount_range() {
        let text = "v=0\r\n\
c=IN IP4 239.1.60.10/15\r\n\
m=audio 5004 RTP/AVP 98\r\n\
a=rtpmap:98 L24/48000/2\r\n\
a=framecount:48-192\r\n\
a=ts-refclk:ptp=IEEE1588-2008:00-00-00-00-00-00-00-01:0\r\n";
        let parsed = parse_sdp(text).unwrap();
        assert_eq!(parsed.frame_size, 48);
    }
}
