// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Human-readable status for `obake-ravenna status`.

use crate::ptp::PtpState;
use crate::qualify::{DownReason, StreamState};

/// Snapshot of PTP lock and every configured stream's up/down state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusReport {
    /// Whether `ptp4l` reports FOLLOWER (locked).
    pub locked: bool,
    /// Local PTP domain number.
    pub domain: u8,
    /// One line per configured source or sink.
    pub streams: Vec<StreamStatusLine>,
}

/// One stream's row in a [`StatusReport`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamStatusLine {
    /// `"source"` or `"sink"`.
    pub kind: &'static str,
    /// Configured stream name.
    pub name: String,
    /// True when the stream has an active PipeWire RTP node.
    pub active: bool,
    /// [`DownReason`] when inactive.
    pub reason: Option<DownReason>,
}

/// Build a status report from the latest PTP state and qualify results.
pub fn build_report(ptp: &PtpState, states: &[StreamState]) -> StatusReport {
    let mut streams = Vec::new();
    for state in states {
        match state {
            StreamState::SourceUp(s) => streams.push(StreamStatusLine {
                kind: "source",
                name: s.name.clone(),
                active: true,
                reason: None,
            }),
            StreamState::SourceDown { name, reason } => streams.push(StreamStatusLine {
                kind: "source",
                name: name.clone(),
                active: false,
                reason: Some(*reason),
            }),
            StreamState::SinkUp(s) => streams.push(StreamStatusLine {
                kind: "sink",
                name: s.name.clone(),
                active: true,
                reason: None,
            }),
            StreamState::SinkDown { name, reason } => streams.push(StreamStatusLine {
                kind: "sink",
                name: name.clone(),
                active: false,
                reason: Some(*reason),
            }),
        }
    }
    StatusReport {
        locked: ptp.locked,
        domain: ptp.domain,
        streams,
    }
}

/// Format a [`StatusReport`] for stdout / the status Unix socket.
pub fn format_report(report: &StatusReport) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "ptp: {} domain {}\n",
        if report.locked { "locked" } else { "unlocked" },
        report.domain
    ));
    for s in &report.streams {
        if s.active {
            out.push_str(&format!("{} {}: active\n", s.kind, s.name));
        } else {
            let reason = s.reason.map(|r| r.as_str()).unwrap_or("unknown");
            out.push_str(&format!("{} {}: held-down ({reason})\n", s.kind, s.name));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ptp::PortState;
    use crate::qualify::DownReason;

    #[test]
    fn prints_unlocked_source() {
        let ptp = PtpState {
            locked: false,
            domain: 0,
            port_state: PortState::Listening,
            gmid: None,
            master_offset_ns: None,
        };
        let states = [StreamState::SourceDown {
            name: "obake-out-1".into(),
            reason: DownReason::Unlocked,
        }];
        let text = format_report(&build_report(&ptp, &states));
        assert!(text.contains("ptp: unlocked"));
        assert!(text.contains("source obake-out-1: held-down (unlocked)"));
    }

    #[test]
    fn prints_lock_hold_while_ptp_locked() {
        let ptp = PtpState {
            locked: true,
            domain: 0,
            port_state: PortState::Follower,
            gmid: Some([1, 2, 3, 4, 5, 6, 7, 8]),
            master_offset_ns: Some(0),
        };
        let states = [StreamState::SourceDown {
            name: "obake-out-1".into(),
            reason: DownReason::LockHold,
        }];
        let text = format_report(&build_report(&ptp, &states));
        assert!(text.contains("ptp: locked"));
        assert!(text.contains("source obake-out-1: held-down (lock-hold)"));
        assert!(!text.contains("held-down (unlocked)"));
    }

    #[test]
    fn prints_offset_high_while_ptp_locked() {
        let ptp = PtpState {
            locked: true,
            domain: 0,
            port_state: PortState::Follower,
            gmid: Some([1, 2, 3, 4, 5, 6, 7, 8]),
            master_offset_ns: Some(3_900_000_000),
        };
        let states = [StreamState::SourceDown {
            name: "obake-out-1".into(),
            reason: DownReason::OffsetHigh,
        }];
        let text = format_report(&build_report(&ptp, &states));
        assert!(text.contains("ptp: locked"));
        assert!(text.contains("source obake-out-1: held-down (offset-high)"));
        assert!(!text.contains("held-down (unlocked)"));
        assert!(!text.contains("held-down (lock-hold)"));
    }

    #[test]
    fn prints_mtu_and_payload_type_reasons() {
        let ptp = PtpState {
            locked: true,
            domain: 0,
            port_state: PortState::Follower,
            gmid: Some([1, 2, 3, 4, 5, 6, 7, 8]),
            master_offset_ns: Some(0),
        };
        let states = [
            StreamState::SourceDown {
                name: "wide".into(),
                reason: DownReason::PayloadTooLarge,
            },
            StreamState::SourceDown {
                name: "bad-pt".into(),
                reason: DownReason::BadPayloadType,
            },
        ];
        let text = format_report(&build_report(&ptp, &states));
        assert!(text.contains("held-down (payload-too-large)"));
        assert!(text.contains("held-down (bad-payload-type)"));
    }
}
