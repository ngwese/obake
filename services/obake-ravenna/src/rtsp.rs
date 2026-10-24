// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! RTSP DESCRIBE server for transmitting RAVENNA sources.
//!
//! Listens on [`RTSP_PORT`] and answers `OPTIONS` / `DESCRIBE` for
//! `/by-name/<stream>` from the shared qualify snapshot. Does not implement
//! SETUP/PLAY; peers only need SDP to join the multicast.

use crate::qualify::{QualifiedSource, StreamState};
use crate::sdp;
use log::{debug, info, warn};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// TCP port advertised via mDNS and bound by the DESCRIBE listener.
pub const RTSP_PORT: u16 = 8554;

/// Path used for peers that DESCRIBE a source by configured name.
pub fn by_name_path(stream: &str) -> String {
    format!("/by-name/{stream}")
}

/// Extract the configured stream name from a `/by-name/...` DESCRIBE path.
pub fn stream_name_from_path(path: &str) -> Option<&str> {
    path.strip_prefix("/by-name/")
        .filter(|s| !s.is_empty() && !s.contains('/'))
}

/// Snapshot of SDP bodies available for DESCRIBE.
#[derive(Debug, Default, Clone)]
pub struct DescribeTable {
    /// Map of configured source name → SDP body text.
    by_name: HashMap<String, String>,
}

impl DescribeTable {
    /// Build a table containing only currently transmitting (`SourceUp`) streams.
    pub fn from_states(states: &[StreamState]) -> Self {
        let mut by_name = HashMap::new();
        for state in states {
            if let StreamState::SourceUp(src) = state {
                by_name.insert(src.name.clone(), sdp_for_source(src));
            }
        }
        Self { by_name }
    }

    /// Look up the SDP body for a DESCRIBE path, if the source is up.
    pub fn describe(&self, path: &str) -> Option<&str> {
        let name = stream_name_from_path(path)?;
        self.by_name.get(name).map(|s| s.as_str())
    }
}

/// Render SDP for one qualified source using its configured name as session.
fn sdp_for_source(src: &QualifiedSource) -> String {
    sdp::build_source_sdp(src, &src.name)
}

/// Shared table the RTSP server reads on each DESCRIBE.
pub type SharedDescribeTable = Arc<Mutex<DescribeTable>>;

/// Create an empty shared DESCRIBE table for the daemon and RTSP task.
pub fn new_shared_table() -> SharedDescribeTable {
    Arc::new(Mutex::new(DescribeTable::default()))
}

/// Replace the shared table contents from the latest qualify results.
pub fn update_table(table: &SharedDescribeTable, states: &[StreamState]) {
    let next = DescribeTable::from_states(states);
    *table.lock().unwrap() = next;
}

/// Build a DESCRIBE 200 response, or None when the source is held down / unknown.
pub fn describe_response(table: &DescribeTable, path: &str, cseq: u32) -> Option<String> {
    let body = table.describe(path)?;
    Some(format!(
        "RTSP/1.0 200 OK\r\n\
CSeq: {cseq}\r\n\
Content-Type: application/sdp\r\n\
Content-Length: {}\r\n\
\r\n\
{body}",
        body.len()
    ))
}

/// Bind `0.0.0.0:RTSP_PORT` and serve DESCRIBE until the task is cancelled.
pub async fn serve(table: SharedDescribeTable) -> Result<(), std::io::Error> {
    let listener = TcpListener::bind(("0.0.0.0", RTSP_PORT)).await?;
    info!("startup: RTSP DESCRIBE listening on 0.0.0.0:{RTSP_PORT}");
    loop {
        let (stream, peer) = listener.accept().await?;
        debug!("rtsp: accept from {peer}");
        let table = Arc::clone(&table);
        tokio::spawn(async move {
            if let Err(e) = handle_client(stream, &table).await {
                debug!("rtsp: client {peer} ended: {e}");
            }
        });
    }
}

async fn handle_client(
    mut stream: TcpStream,
    table: &SharedDescribeTable,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    loop {
        let req = match read_request(&mut stream).await? {
            Some(r) => r,
            None => return Ok(()),
        };
        debug!("rtsp: {} {} CSeq={}", req.method, req.path, req.cseq);
        let response = match req.method.as_str() {
            "OPTIONS" => options_response(req.cseq),
            "DESCRIBE" => {
                let snapshot = table.lock().unwrap().clone();
                match describe_response(&snapshot, &req.path, req.cseq) {
                    Some(ok) => ok,
                    None => {
                        info!("rtsp: DESCRIBE {} -> 404", req.path);
                        not_found_response(req.cseq)
                    }
                }
            }
            other => {
                warn!("rtsp: unsupported method {other}");
                method_not_allowed_response(req.cseq)
            }
        };
        stream.write_all(response.as_bytes()).await?;
        if req.method == "DESCRIBE" {
            // One-shot DESCRIBE is enough for SDP retrieval; close after.
            return Ok(());
        }
    }
}

#[derive(Debug)]
struct RtspRequest {
    method: String,
    /// Absolute path, e.g. `/by-name/obake-out-1`.
    path: String,
    cseq: u32,
}

async fn read_request(
    stream: &mut TcpStream,
) -> Result<Option<RtspRequest>, Box<dyn std::error::Error + Send + Sync>> {
    let mut buf = Vec::with_capacity(1024);
    let mut tmp = [0u8; 512];
    loop {
        let n = stream.read(&mut tmp).await?;
        if n == 0 {
            return Ok(None);
        }
        buf.extend_from_slice(&tmp[..n]);
        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
        if buf.len() > 16 * 1024 {
            return Err("rtsp request too large".into());
        }
    }
    let text = String::from_utf8_lossy(&buf);
    Ok(Some(parse_request(&text)?))
}

fn parse_request(text: &str) -> Result<RtspRequest, Box<dyn std::error::Error + Send + Sync>> {
    let mut lines = text.split("\r\n");
    let request_line = lines.next().ok_or("empty RTSP request")?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().ok_or("missing method")?.to_string();
    let uri = parts.next().ok_or("missing URI")?;
    let path = uri_path(uri).ok_or("invalid RTSP URI")?.to_string();

    let mut cseq = 1u32;
    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some(rest) = line
            .strip_prefix("CSeq:")
            .or_else(|| line.strip_prefix("Cseq:"))
        {
            if let Ok(n) = rest.trim().parse() {
                cseq = n;
            }
        }
    }
    Ok(RtspRequest { method, path, cseq })
}

/// Extract the absolute path from an RTSP request-URI.
fn uri_path(uri: &str) -> Option<&str> {
    if uri.starts_with('/') {
        return Some(uri.split('?').next().unwrap_or(uri));
    }
    let rest = uri.strip_prefix("rtsp://")?;
    let path = rest.find('/')?;
    Some(rest[path..].split('?').next().unwrap_or(&rest[path..]))
}

fn options_response(cseq: u32) -> String {
    format!(
        "RTSP/1.0 200 OK\r\n\
CSeq: {cseq}\r\n\
Public: OPTIONS, DESCRIBE\r\n\
\r\n"
    )
}

fn not_found_response(cseq: u32) -> String {
    format!(
        "RTSP/1.0 404 Not Found\r\n\
CSeq: {cseq}\r\n\
\r\n"
    )
}

fn method_not_allowed_response(cseq: u32) -> String {
    format!(
        "RTSP/1.0 405 Method Not Allowed\r\n\
CSeq: {cseq}\r\n\
Public: OPTIONS, DESCRIBE\r\n\
\r\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Format;
    use crate::destination::MulticastDestination;
    use crate::qualify::{DownReason, QualifiedSource, StreamState};

    fn up_source() -> StreamState {
        StreamState::SourceUp(QualifiedSource {
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
        })
    }

    #[test]
    fn path_builder() {
        assert_eq!(by_name_path("PGM-1"), "/by-name/PGM-1");
        assert_eq!(stream_name_from_path("/by-name/PGM-1"), Some("PGM-1"));
        assert_eq!(stream_name_from_path("/other"), None);
    }

    #[test]
    fn uri_path_from_absolute_and_full() {
        assert_eq!(uri_path("/by-name/obake-out-1"), Some("/by-name/obake-out-1"));
        assert_eq!(
            uri_path("rtsp://bata.local:8554/by-name/obake-out-1"),
            Some("/by-name/obake-out-1")
        );
    }

    #[test]
    fn parse_describe_extracts_cseq_and_path() {
        let raw = "DESCRIBE rtsp://bata.local/by-name/obake-out-1 RTSP/1.0\r\n\
CSeq: 7\r\n\
Accept: application/sdp\r\n\
\r\n";
        let req = parse_request(raw).unwrap();
        assert_eq!(req.method, "DESCRIBE");
        assert_eq!(req.path, "/by-name/obake-out-1");
        assert_eq!(req.cseq, 7);
    }

    #[test]
    fn describe_transmitting_source() {
        let table = DescribeTable::from_states(&[up_source()]);
        let resp = describe_response(&table, "/by-name/obake-out-1", 1).unwrap();
        assert!(resp.starts_with("RTSP/1.0 200 OK"));
        assert!(resp.contains("CSeq: 1"));
        assert!(resp.contains("L24/48000/8"));
        assert!(resp.contains("239.1.60.50"));
        assert!(resp.contains("RTP/AVP 104"));
    }

    #[test]
    fn describe_held_down_source_returns_none() {
        let states = [StreamState::SourceDown {
            name: "obake-out-1".into(),
            reason: DownReason::Unlocked,
        }];
        let table = DescribeTable::from_states(&states);
        assert!(describe_response(&table, "/by-name/obake-out-1", 1).is_none());
    }
}
