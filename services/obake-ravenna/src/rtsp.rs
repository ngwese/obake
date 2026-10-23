// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! RTSP DESCRIBE path and a minimal in-process server for tests.

use crate::qualify::{QualifiedSource, StreamState};
use crate::sdp;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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

/// Build a DESCRIBE response body, or None when the source is held down /
/// unknown.
pub fn describe_response(table: &DescribeTable, path: &str) -> Option<String> {
    let body = table.describe(path)?;
    Some(format!(
        "RTSP/1.0 200 OK\r\n\
Content-Type: application/sdp\r\n\
Content-Length: {}\r\n\
\r\n\
{body}",
        body.len()
    ))
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
            destination: MulticastDestination::parse("239.69.1.10:5004").unwrap(),
            ttl: 15,
            rate: 48_000,
            domain: 0,
        })
    }

    #[test]
    fn path_builder() {
        assert_eq!(by_name_path("PGM-1"), "/by-name/PGM-1");
        assert_eq!(stream_name_from_path("/by-name/PGM-1"), Some("PGM-1"));
        assert_eq!(stream_name_from_path("/other"), None);
    }

    #[test]
    fn describe_transmitting_source() {
        let table = DescribeTable::from_states(&[up_source()]);
        let resp = describe_response(&table, "/by-name/obake-out-1").unwrap();
        assert!(resp.starts_with("RTSP/1.0 200 OK"));
        assert!(resp.contains("L24/48000/8"));
        assert!(resp.contains("239.69.1.10"));
    }

    #[test]
    fn describe_held_down_source_returns_none() {
        let states = [StreamState::SourceDown {
            name: "obake-out-1".into(),
            reason: DownReason::Unlocked,
        }];
        let table = DescribeTable::from_states(&states);
        assert!(describe_response(&table, "/by-name/obake-out-1").is_none());
    }
}
