// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! One-shot RAVENNA network browse: path selection, grouping, and report text.
//!
//! Network I/O (Avahi / RTSP) lives elsewhere; this module stays unit-testable
//! without a bus or peer.

use crate::sdp::StreamSdp;
use std::collections::HashMap;
use std::net::Ipv4Addr;

/// A DNS-SD peer resolved to an IPv4 endpoint (device or session).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPeer {
    /// DNS-SD instance name.
    pub name: String,
    /// Resolved unicast IPv4 from Avahi.
    pub address: Ipv4Addr,
    /// RTSP TCP port.
    pub port: u16,
    /// Raw TXT records (`key=value` bytes).
    pub txt: Vec<Vec<u8>>,
}

/// Outcome of DESCRIBE + SDP parse for one session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionSummary {
    /// Parsed media parameters.
    Ok(StreamSdp),
    /// Short reason (`describe-failed`, `sdp-invalid`, …).
    Failed(&'static str),
}

/// One session row in a browse report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRow {
    /// DNS-SD session instance name.
    pub name: String,
    /// Media summary or failure reason.
    pub summary: SessionSummary,
}

/// Device section: node record plus sessions sharing its endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceSection {
    /// DNS-SD node instance name.
    pub name: String,
    /// Resolved unicast IPv4.
    pub address: Ipv4Addr,
    /// RTSP TCP port.
    pub port: u16,
    /// Sessions grouped under this device.
    pub sessions: Vec<SessionRow>,
}

/// Orphan sessions sharing an endpoint with no matching device record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnassignedSection {
    /// Resolved unicast IPv4.
    pub address: Ipv4Addr,
    /// RTSP TCP port.
    pub port: u16,
    /// Sessions at that endpoint.
    pub sessions: Vec<SessionRow>,
}

/// Full browse report ready for formatting.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BrowseReport {
    /// Devices with zero or more sessions.
    pub devices: Vec<DeviceSection>,
    /// Sessions whose endpoint matched no device.
    pub unassigned: Vec<UnassignedSection>,
}

/// Choose the RTSP DESCRIBE path for a session from TXT or the instance name.
///
/// Prefers a TXT record `path=<value>`. Absolute values are used as-is; relative
/// values get a leading `/`. With no `path` key, returns `/by-name/<name>`.
///
/// The returned path is **not** percent-encoded; pass it through
/// [`encode_rtsp_path`] before putting it on an RTSP request line.
pub fn describe_path(session_name: &str, txt: &[Vec<u8>]) -> String {
    for rec in txt {
        let Ok(s) = std::str::from_utf8(rec) else {
            continue;
        };
        let Some(value) = s.strip_prefix("path=") else {
            continue;
        };
        if value.is_empty() {
            continue;
        }
        if value.starts_with('/') {
            return value.to_string();
        }
        return format!("/{value}");
    }
    format!("/by-name/{session_name}")
}

/// Percent-encode an RTSP absolute path for a request-URI (RFC 3986).
///
/// Splits on `/`, encodes each segment (spaces, parentheses, etc.), and
/// rejoins so structural slashes stay literal. Merging/Anubis reject request
/// lines that contain raw spaces in the path.
pub fn encode_rtsp_path(path: &str) -> String {
    let segments = path.trim_start_matches('/');
    format!("/{}", encode_rtsp_path_segments(segments))
}

fn encode_rtsp_path_segments(path: &str) -> String {
    path.split('/')
        .map(encode_path_segment)
        .collect::<Vec<_>>()
        .join("/")
}

fn encode_path_segment(seg: &str) -> String {
    let mut out = String::with_capacity(seg.len());
    for b in seg.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push('%');
                out.push(nibble(b >> 4));
                out.push(nibble(b & 0x0f));
            }
        }
    }
    out
}

fn nibble(n: u8) -> char {
    char::from(b"0123456789ABCDEF"[n as usize])
}

fn endpoint_key(address: Ipv4Addr, port: u16) -> (Ipv4Addr, u16) {
    (address, port)
}

/// Group session peers under device peers that share the same IPv4 and port.
///
/// Returns `(matched device -> sessions, orphan sessions)`. Device order follows
/// `devices`; session order within a group follows `sessions`.
pub fn group_by_endpoint(
    devices: &[ResolvedPeer],
    sessions: &[ResolvedPeer],
) -> (Vec<(ResolvedPeer, Vec<ResolvedPeer>)>, Vec<ResolvedPeer>) {
    let mut by_endpoint: HashMap<(Ipv4Addr, u16), Vec<ResolvedPeer>> = HashMap::new();
    for session in sessions {
        by_endpoint
            .entry(endpoint_key(session.address, session.port))
            .or_default()
            .push(session.clone());
    }

    let mut matched = Vec::new();
    let mut claimed: HashMap<(Ipv4Addr, u16), ()> = HashMap::new();
    for device in devices {
        let key = endpoint_key(device.address, device.port);
        let sess = by_endpoint.remove(&key).unwrap_or_default();
        claimed.insert(key, ());
        matched.push((device.clone(), sess));
    }

    let mut orphans = Vec::new();
    for session in sessions {
        let key = endpoint_key(session.address, session.port);
        if claimed.contains_key(&key) {
            continue;
        }
        orphans.push(session.clone());
    }
    (matched, orphans)
}

/// Build a [`BrowseReport`] from grouped peers and per-session summaries.
///
/// `summaries` is keyed by session DNS-SD instance name. Missing entries become
/// `describe-failed`.
pub fn build_report(
    matched: &[(ResolvedPeer, Vec<ResolvedPeer>)],
    orphans: &[ResolvedPeer],
    summaries: &HashMap<String, SessionSummary>,
) -> BrowseReport {
    let mut devices = Vec::new();
    for (device, sessions) in matched {
        let rows = sessions
            .iter()
            .map(|s| SessionRow {
                name: s.name.clone(),
                summary: summaries
                    .get(&s.name)
                    .cloned()
                    .unwrap_or(SessionSummary::Failed("describe-failed")),
            })
            .collect();
        devices.push(DeviceSection {
            name: device.name.clone(),
            address: device.address,
            port: device.port,
            sessions: rows,
        });
    }

    let mut orphan_groups: HashMap<(Ipv4Addr, u16), Vec<SessionRow>> = HashMap::new();
    let mut orphan_order: Vec<(Ipv4Addr, u16)> = Vec::new();
    for session in orphans {
        let key = endpoint_key(session.address, session.port);
        if !orphan_groups.contains_key(&key) {
            orphan_order.push(key);
            orphan_groups.insert(key, Vec::new());
        }
        orphan_groups.get_mut(&key).unwrap().push(SessionRow {
            name: session.name.clone(),
            summary: summaries
                .get(&session.name)
                .cloned()
                .unwrap_or(SessionSummary::Failed("describe-failed")),
        });
    }
    let unassigned = orphan_order
        .into_iter()
        .map(|(address, port)| UnassignedSection {
            address,
            port,
            sessions: orphan_groups.remove(&(address, port)).unwrap_or_default(),
        })
        .collect();

    BrowseReport {
        devices,
        unassigned,
    }
}

/// Format a [`BrowseReport`] for stdout.
pub fn format_report(report: &BrowseReport) -> String {
    format_report_inner(report, None)
}

/// Format a browse report, optionally dumping raw SDP after each session.
///
/// `raw_sdp` maps session DNS-SD instance name → unmodified DESCRIBE body.
pub fn format_report_with_sdp_dumps(
    report: &BrowseReport,
    raw_sdp: &HashMap<String, String>,
) -> String {
    format_report_inner(report, Some(raw_sdp))
}

fn format_report_inner(
    report: &BrowseReport,
    raw_sdp: Option<&HashMap<String, String>>,
) -> String {
    let mut out = String::new();
    for device in &report.devices {
        out.push_str(&format!(
            "device  {}  {}:{}\n",
            device.name, device.address, device.port
        ));
        for session in &device.sessions {
            out.push_str(&format_session_line(session));
            append_sdp_dump(&mut out, &session.name, raw_sdp);
        }
    }
    for section in &report.unassigned {
        out.push_str(&format!(
            "unassigned  {}:{}\n",
            section.address, section.port
        ));
        for session in &section.sessions {
            out.push_str(&format_session_line(session));
            append_sdp_dump(&mut out, &session.name, raw_sdp);
        }
    }
    out
}

fn append_sdp_dump(out: &mut String, name: &str, raw_sdp: Option<&HashMap<String, String>>) {
    let Some(map) = raw_sdp else {
        return;
    };
    let Some(body) = map.get(name) else {
        return;
    };
    out.push_str("---- SDP begin ----\n");
    out.push_str(body);
    if !body.ends_with('\n') {
        out.push('\n');
    }
    out.push_str("---- SDP end ----\n");
}

fn format_session_line(session: &SessionRow) -> String {
    match &session.summary {
        SessionSummary::Ok(sdp) => format!(
            "  session  {}  {} {}/{}  frames={}  {}:{}\n",
            session.name,
            sdp.format.as_str(),
            sdp.rate,
            sdp.channels,
            sdp.frame_size,
            sdp.group,
            sdp.port
        ),
        SessionSummary::Failed(reason) => {
            format!("  session  {}  {reason}\n", session.name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Format;
    use std::net::Ipv4Addr;

    fn peer(name: &str, ip: [u8; 4], port: u16, txt: Vec<Vec<u8>>) -> ResolvedPeer {
        ResolvedPeer {
            name: name.into(),
            address: Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]),
            port,
            txt,
        }
    }

    fn sample_sdp() -> StreamSdp {
        StreamSdp {
            group: Ipv4Addr::new(239, 1, 0, 1),
            port: 5004,
            rate: 48_000,
            channels: 8,
            format: Format::L24,
            frame_size: 48,
            domain: 0,
            payload_type: 104,
        }
    }

    #[test]
    fn describe_path_uses_txt_when_present() {
        let txt = vec![b"path=/by-name/obake-out-1".to_vec()];
        assert_eq!(
            describe_path("ignored", &txt),
            "/by-name/obake-out-1"
        );
    }

    #[test]
    fn describe_path_prefixes_relative_txt() {
        let txt = vec![b"path=by-name/PGM-1".to_vec()];
        assert_eq!(describe_path("PGM-1", &txt), "/by-name/PGM-1");
    }

    #[test]
    fn describe_path_defaults_to_by_name() {
        assert_eq!(describe_path("PGM-1", &[]), "/by-name/PGM-1");
        let other = vec![b"foo=bar".to_vec()];
        assert_eq!(describe_path("PGM-1", &other), "/by-name/PGM-1");
    }

    #[test]
    fn encode_rtsp_path_encodes_space_and_parens() {
        assert_eq!(
            encode_rtsp_path("/by-name/Anubis_610267_Combo 1/2"),
            "/by-name/Anubis_610267_Combo%201/2"
        );
        assert_eq!(
            encode_rtsp_path("/by-name/ASIO (on caxixi)_AUX1-1"),
            "/by-name/ASIO%20%28on%20caxixi%29_AUX1-1"
        );
        assert_eq!(encode_rtsp_path("/by-name/safe"), "/by-name/safe");
    }

    #[test]
    fn group_matches_sessions_to_device_endpoint() {
        let devices = vec![peer("Anubis", [192, 168, 60, 10], 8554, vec![])];
        let sessions = vec![
            peer("PGM-1", [192, 168, 60, 10], 8554, vec![]),
            peer("Cue", [192, 168, 60, 10], 8554, vec![]),
        ];
        let (matched, orphans) = group_by_endpoint(&devices, &sessions);
        assert!(orphans.is_empty());
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].0.name, "Anubis");
        assert_eq!(matched[0].1.len(), 2);
        assert_eq!(matched[0].1[0].name, "PGM-1");
        assert_eq!(matched[0].1[1].name, "Cue");
    }

    #[test]
    fn group_keeps_orphan_sessions() {
        let devices = vec![peer("bata", [192, 168, 60, 50], 8554, vec![])];
        let sessions = vec![peer("Lone", [192, 168, 60, 99], 8554, vec![])];
        let (matched, orphans) = group_by_endpoint(&devices, &sessions);
        assert_eq!(matched.len(), 1);
        assert!(matched[0].1.is_empty());
        assert_eq!(orphans.len(), 1);
        assert_eq!(orphans[0].name, "Lone");
    }

    #[test]
    fn format_success_and_failure_rows() {
        let report = BrowseReport {
            devices: vec![DeviceSection {
                name: "Anubis-Main".into(),
                address: Ipv4Addr::new(192, 168, 60, 10),
                port: 8554,
                sessions: vec![
                    SessionRow {
                        name: "PGM-1".into(),
                        summary: SessionSummary::Ok(sample_sdp()),
                    },
                    SessionRow {
                        name: "Cue".into(),
                        summary: SessionSummary::Failed("describe-failed"),
                    },
                    SessionRow {
                        name: "Bad".into(),
                        summary: SessionSummary::Failed("sdp-invalid"),
                    },
                ],
            }],
            unassigned: vec![],
        };
        let text = format_report(&report);
        assert!(text.contains("device  Anubis-Main  192.168.60.10:8554\n"));
        assert!(text.contains(
            "  session  PGM-1  L24 48000/8  frames=48  239.1.0.1:5004\n"
        ));
        assert!(text.contains("  session  Cue  describe-failed\n"));
        assert!(text.contains("  session  Bad  sdp-invalid\n"));
    }

    #[test]
    fn format_unassigned_section() {
        let report = BrowseReport {
            devices: vec![],
            unassigned: vec![UnassignedSection {
                address: Ipv4Addr::new(192, 168, 60, 99),
                port: 8554,
                sessions: vec![SessionRow {
                    name: "Lone".into(),
                    summary: SessionSummary::Ok(sample_sdp()),
                }],
            }],
        };
        let text = format_report(&report);
        assert!(text.contains("unassigned  192.168.60.99:8554\n"));
        assert!(text.contains("  session  Lone  L24"));
    }

    #[test]
    fn format_dumps_raw_sdp_after_session() {
        let report = BrowseReport {
            devices: vec![DeviceSection {
                name: "dev".into(),
                address: Ipv4Addr::new(192, 168, 60, 10),
                port: 80,
                sessions: vec![SessionRow {
                    name: "PGM-1".into(),
                    summary: SessionSummary::Ok(sample_sdp()),
                }],
            }],
            unassigned: vec![],
        };
        let mut raw = HashMap::new();
        raw.insert("PGM-1".into(), "v=0\r\ns=PGM-1\r\n".into());
        let text = format_report_with_sdp_dumps(&report, &raw);
        assert!(text.contains("  session  PGM-1  L24"));
        assert!(text.contains("---- SDP begin ----\n"));
        assert!(text.contains("v=0\r\ns=PGM-1\r\n"));
        assert!(text.contains("---- SDP end ----\n"));
    }
}
