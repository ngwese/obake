// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Read `ptp4l` lock state over the read-only management Unix socket.
//!
//! Speaks IEEE 1588 management TLVs in-process (same messages `pmc -u`
//! would send). Never spawns a helper and never adjusts a clock.

use log::debug;
use std::io;
use std::os::unix::net::UnixDatagram;
use std::path::{Path, PathBuf};
use std::time::Duration;

const MSG_MANAGEMENT: u8 = 0x0d;
const PTP_VERSION: u8 = 0x12; // minor 1 | major 2, matching linuxptp 4.x
const TLV_MANAGEMENT: u16 = 0x0001;
const MGMT_ACTION_GET: u8 = 0x00;
const MGMT_ACTION_RESPONSE: u8 = 0x02;

const MID_DOMAIN: u16 = 0x2007;
const MID_PORT_DATA_SET: u16 = 0x2004;

const MGMT_PREFIX_LEN: usize = 48; // header + targetPortIdentity + hops/action
const TLV_HEADER_LEN: usize = 6; // type + length + id
const GET_REQUEST_LEN: usize = MGMT_PREFIX_LEN + TLV_HEADER_LEN;

/// Latest PTP lock view used by qualify and status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PtpState {
    /// True when [`PortState`] is SLAVE (daemon treats that as locked).
    pub locked: bool,
    /// Domain number from the DOMAIN management ID.
    pub domain: u8,
    /// Raw port state from PORT_DATA_SET.
    pub port_state: PortState,
}

impl PtpState {
    /// Conservative default when the UDS query fails or is unavailable.
    pub fn unlocked() -> Self {
        Self {
            locked: false,
            domain: 0,
            port_state: PortState::Unknown,
        }
    }
}

/// IEEE 1588 port state values (linuxptp `portState` byte).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PortState {
    Initializing = 1,
    Faulty = 2,
    Disabled = 3,
    Listening = 4,
    PreMaster = 5,
    Master = 6,
    Passive = 7,
    Uncalibrated = 8,
    /// Synchronized to a grandmaster; daemon requires this for stream up.
    Slave = 9,
    /// Unrecognized or missing value.
    Unknown = 0,
}

impl PortState {
    /// Map the on-wire `portState` byte to an enum variant.
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => PortState::Initializing,
            2 => PortState::Faulty,
            3 => PortState::Disabled,
            4 => PortState::Listening,
            5 => PortState::PreMaster,
            6 => PortState::Master,
            7 => PortState::Passive,
            8 => PortState::Uncalibrated,
            9 => PortState::Slave,
            _ => PortState::Unknown,
        }
    }

    /// Parse a `pmc`-style text token (`SLAVE`, `LISTENING`, …).
    pub fn parse(text: &str) -> Self {
        match text.trim() {
            "INITIALIZING" => PortState::Initializing,
            "FAULTY" => PortState::Faulty,
            "DISABLED" => PortState::Disabled,
            "LISTENING" => PortState::Listening,
            "PRE_MASTER" => PortState::PreMaster,
            "MASTER" => PortState::Master,
            "PASSIVE" => PortState::Passive,
            "UNCALIBRATED" => PortState::Uncalibrated,
            "SLAVE" => PortState::Slave,
            _ => PortState::Unknown,
        }
    }

    /// Whether this state counts as locked for stream qualification.
    pub fn is_locked(self) -> bool {
        matches!(self, PortState::Slave)
    }

    /// Stable name for logs (`SLAVE`, `LISTENING`, …).
    pub fn as_str(self) -> &'static str {
        match self {
            PortState::Initializing => "INITIALIZING",
            PortState::Faulty => "FAULTY",
            PortState::Disabled => "DISABLED",
            PortState::Listening => "LISTENING",
            PortState::PreMaster => "PRE_MASTER",
            PortState::Master => "MASTER",
            PortState::Passive => "PASSIVE",
            PortState::Uncalibrated => "UNCALIBRATED",
            PortState::Slave => "SLAVE",
            PortState::Unknown => "UNKNOWN",
        }
    }
}

/// Build a GET management request (network byte order).
pub fn build_get_request(mgmt_id: u16, sequence_id: u16) -> [u8; GET_REQUEST_LEN] {
    let mut buf = [0u8; GET_REQUEST_LEN];
    // ptp_header
    buf[0] = MSG_MANAGEMENT;
    buf[1] = PTP_VERSION;
    buf[2..4].copy_from_slice(&(GET_REQUEST_LEN as u16).to_be_bytes());
    // domainNumber at 4 left 0
    // flags, correction, reserved already zero
    // sourcePortIdentity (bytes 20..30) left zero
    buf[30..32].copy_from_slice(&sequence_id.to_be_bytes());
    buf[32] = 0x04; // control = management (historical)
    buf[33] = 0x7f; // logMessageInterval

    // targetPortIdentity: all-ones = all clocks/ports (linuxptp pmc default)
    for b in &mut buf[34..44] {
        *b = 0xff;
    }
    buf[44] = 1; // startingBoundaryHops
    buf[45] = 1; // boundaryHops
    buf[46] = MGMT_ACTION_GET;
    // buf[47] reserved

    // MANAGEMENT TLV with empty data
    buf[48..50].copy_from_slice(&TLV_MANAGEMENT.to_be_bytes());
    buf[50..52].copy_from_slice(&2u16.to_be_bytes()); // length = id only
    buf[52..54].copy_from_slice(&mgmt_id.to_be_bytes());
    buf
}

/// Parse a management response; returns (managementId, data payload).
pub fn parse_management_response(msg: &[u8]) -> Result<(u16, &[u8]), String> {
    if msg.len() < MGMT_PREFIX_LEN + TLV_HEADER_LEN {
        return Err("management response too short".into());
    }
    if msg[0] & 0x0f != MSG_MANAGEMENT {
        return Err("not a management message".into());
    }
    let action = msg[46] & 0x0f;
    if action != MGMT_ACTION_RESPONSE {
        return Err(format!("unexpected management action {action}"));
    }
    let tlv_type = u16::from_be_bytes([msg[48], msg[49]]);
    if tlv_type != TLV_MANAGEMENT {
        return Err(format!("unexpected TLV type {tlv_type:#x}"));
    }
    let tlv_len = u16::from_be_bytes([msg[50], msg[51]]) as usize;
    if tlv_len < 2 {
        return Err("TLV length too small".into());
    }
    let id = u16::from_be_bytes([msg[52], msg[53]]);
    let data_len = tlv_len - 2;
    let data_end = MGMT_PREFIX_LEN + TLV_HEADER_LEN + data_len;
    if msg.len() < data_end {
        return Err("TLV data truncated".into());
    }
    Ok((id, &msg[MGMT_PREFIX_LEN + TLV_HEADER_LEN..data_end]))
}

/// Build a RESPONSE with the given management id and data (for fixtures).
pub fn build_response(mgmt_id: u16, sequence_id: u16, data: &[u8]) -> Vec<u8> {
    let tlv_len = 2 + data.len();
    let total = MGMT_PREFIX_LEN + 4 + tlv_len; // type+length + id+data
    let mut buf = vec![0u8; total];
    buf[0] = MSG_MANAGEMENT;
    buf[1] = PTP_VERSION;
    buf[2..4].copy_from_slice(&(total as u16).to_be_bytes());
    buf[30..32].copy_from_slice(&sequence_id.to_be_bytes());
    buf[32] = 0x04;
    buf[33] = 0x7f;
    for b in &mut buf[34..44] {
        *b = 0xff;
    }
    buf[44] = 1;
    buf[45] = 1;
    buf[46] = MGMT_ACTION_RESPONSE;
    buf[48..50].copy_from_slice(&TLV_MANAGEMENT.to_be_bytes());
    buf[50..52].copy_from_slice(&(tlv_len as u16).to_be_bytes());
    buf[52..54].copy_from_slice(&mgmt_id.to_be_bytes());
    buf[54..54 + data.len()].copy_from_slice(data);
    buf
}

/// portDS layout from linuxptp `ds.h` (packed, network order on the wire
/// for multi-byte fields; portState is a single byte).
pub fn parse_port_data_set(data: &[u8]) -> Result<PortState, String> {
    // portIdentity (10) then portState (1)
    if data.len() < 11 {
        return Err("PORT_DATA_SET data too short".into());
    }
    Ok(PortState::from_u8(data[10]))
}

/// Extract `domainNumber` from a DOMAIN management TLV payload.
pub fn parse_domain_data(data: &[u8]) -> Result<u8, String> {
    // management_tlv_datum: val + reserved
    if data.is_empty() {
        return Err("DOMAIN data empty".into());
    }
    Ok(data[0])
}

/// Build portDS-sized payload with the given port state (zeros elsewhere).
pub fn fixture_port_data_set(port_state: PortState) -> [u8; 26] {
    let mut data = [0u8; 26];
    data[10] = port_state as u8;
    data
}

/// DOMAIN TLV payload for unit-test responses.
pub fn fixture_domain_data(domain: u8) -> [u8; 2] {
    [domain, 0]
}

/// Persistent UDS client for `ptp4l`'s read-only management socket.
pub struct PtpManagementClient {
    /// Bound client datagram socket.
    sock: UnixDatagram,
    /// Path to `ptp4l`'s read-only management socket.
    server: PathBuf,
    /// This process's bound pathname (removed on drop).
    client_path: PathBuf,
    /// Next PTP `sequenceId` for GET requests.
    sequence: u16,
}

impl PtpManagementClient {
    /// Bind a datagram client socket and remember the server path.
    /// Does not connect yet; each query sendto/recvfrom the server.
    pub fn open(server: impl AsRef<Path>, client_path: impl AsRef<Path>) -> io::Result<Self> {
        let client_path = client_path.as_ref().to_path_buf();
        let server = server.as_ref().to_path_buf();
        debug!(
            "ptp: opening management client server={} client={}",
            server.display(),
            client_path.display()
        );
        let _ = std::fs::remove_file(&client_path);
        if let Some(parent) = client_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let sock = UnixDatagram::bind(&client_path)?;
        sock.set_read_timeout(Some(Duration::from_millis(200)))?;
        sock.set_write_timeout(Some(Duration::from_millis(200)))?;
        Ok(Self {
            sock,
            server,
            client_path,
            sequence: 1,
        })
    }

    /// Path of the `ptp4l` management socket this client queries.
    pub fn server_path(&self) -> &Path {
        &self.server
    }

    /// Allocate the next request sequence id.
    fn next_seq(&mut self) -> u16 {
        let s = self.sequence;
        self.sequence = self.sequence.wrapping_add(1);
        s
    }

    /// Send one GET and return the response TLV data payload.
    fn transact(&mut self, mgmt_id: u16) -> Result<Vec<u8>, String> {
        let seq = self.next_seq();
        debug!("ptp: GET mid={mgmt_id:#x} seq={seq}");
        let req = build_get_request(mgmt_id, seq);
        self.sock
            .send_to(&req, &self.server)
            .map_err(|e| e.to_string())?;
        let mut buf = [0u8; 512];
        let (n, _) = self.sock.recv_from(&mut buf).map_err(|e| e.to_string())?;
        let (id, data) = parse_management_response(&buf[..n])?;
        if id != mgmt_id {
            return Err(format!("expected mid {mgmt_id:#x}, got {id:#x}"));
        }
        debug!("ptp: RESPONSE mid={id:#x} data_len={}", data.len());
        Ok(data.to_vec())
    }

    /// Query PORT_DATA_SET and DOMAIN. On any I/O or parse failure, unlocked.
    pub fn query_state(&mut self) -> PtpState {
        let port_data = match self.transact(MID_PORT_DATA_SET) {
            Ok(d) => d,
            Err(e) => {
                debug!("ptp: PORT_DATA_SET failed: {e}; treating as unlocked");
                return PtpState::unlocked();
            }
        };
        let port_state = match parse_port_data_set(&port_data) {
            Ok(s) => s,
            Err(e) => {
                debug!("ptp: parse PORT_DATA_SET failed: {e}; treating as unlocked");
                return PtpState::unlocked();
            }
        };
        let domain = match self.transact(MID_DOMAIN) {
            Ok(d) => parse_domain_data(&d).unwrap_or_else(|e| {
                debug!("ptp: parse DOMAIN failed: {e}; using domain 0");
                0
            }),
            Err(e) => {
                debug!("ptp: DOMAIN failed: {e}; using domain 0");
                0
            }
        };
        let state = PtpState {
            locked: port_state.is_locked(),
            domain,
            port_state,
        };
        debug!(
            "ptp: query result locked={} domain={} port_state={}",
            state.locked,
            state.domain,
            state.port_state.as_str()
        );
        state
    }
}

impl Drop for PtpManagementClient {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.client_path);
    }
}

/// Read the interface section header from a `ptp4l` config (first non-global).
pub fn interface_from_ptp4l_cfg(text: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim();
        if let Some(name) = line.strip_prefix('[') {
            if let Some(name) = name.strip_suffix(']') {
                if name != "global" {
                    return Some(name.to_string());
                }
            }
        }
    }
    None
}

/// Parse a captured `pmc` text reply (fixtures / operator paste). Not used
/// on the daemon hot path.
pub fn parse_pmc_reply(text: &str) -> PtpState {
    let mut port_state = PortState::Unknown;
    let mut domain = 0u8;
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("portState") {
            let value = rest.trim().trim_start_matches([' ', '\t']);
            port_state = PortState::parse(value);
        } else if let Some(rest) = line.strip_prefix("domainNumber") {
            let value = rest.trim().trim_start_matches([' ', '\t']);
            if let Ok(n) = value.parse() {
                domain = n;
            }
        }
    }
    PtpState {
        locked: port_state.is_locked(),
        domain,
        port_state,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixDatagram;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use tempfile::tempdir;

    #[test]
    fn binary_slave_is_locked() {
        let data = fixture_port_data_set(PortState::Slave);
        let msg = build_response(MID_PORT_DATA_SET, 1, &data);
        let (id, payload) = parse_management_response(&msg).unwrap();
        assert_eq!(id, MID_PORT_DATA_SET);
        let st = parse_port_data_set(payload).unwrap();
        assert_eq!(st, PortState::Slave);
        assert!(st.is_locked());
    }

    #[test]
    fn binary_listening_is_unlocked() {
        let data = fixture_port_data_set(PortState::Listening);
        let msg = build_response(MID_PORT_DATA_SET, 2, &data);
        let (_, payload) = parse_management_response(&msg).unwrap();
        let st = parse_port_data_set(payload).unwrap();
        assert_eq!(st, PortState::Listening);
        assert!(!st.is_locked());
    }

    #[test]
    fn binary_domain_payload() {
        let data = fixture_domain_data(0);
        let msg = build_response(MID_DOMAIN, 3, &data);
        let (id, payload) = parse_management_response(&msg).unwrap();
        assert_eq!(id, MID_DOMAIN);
        assert_eq!(parse_domain_data(payload).unwrap(), 0);
    }

    #[test]
    fn get_request_has_expected_mid() {
        let req = build_get_request(MID_PORT_DATA_SET, 7);
        assert_eq!(req.len(), GET_REQUEST_LEN);
        assert_eq!(req[0] & 0x0f, MSG_MANAGEMENT);
        assert_eq!(u16::from_be_bytes([req[52], req[53]]), MID_PORT_DATA_SET);
        assert_eq!(req[46] & 0x0f, MGMT_ACTION_GET);
    }

    #[test]
    fn uds_client_queries_without_spawning() {
        // Local fake ptp4l: answer PORT_DATA_SET and DOMAIN over UDS.
        // No std::process on this path.
        let dir = tempdir().unwrap();
        let server_path = dir.path().join("ptp4l.ro");
        let client_path = dir.path().join("client.pmc");
        let server = UnixDatagram::bind(&server_path).unwrap();

        let done = Arc::new(Mutex::new(false));
        let done_flag = Arc::clone(&done);
        let server_thread = thread::spawn(move || {
            let mut buf = [0u8; 512];
            for _ in 0..2 {
                let (n, src) = server.recv_from(&mut buf).unwrap();
                let _ = n;
                let id = u16::from_be_bytes([buf[52], buf[53]]);
                let seq = u16::from_be_bytes([buf[30], buf[31]]);
                let resp = if id == MID_PORT_DATA_SET {
                    build_response(id, seq, &fixture_port_data_set(PortState::Slave))
                } else {
                    build_response(id, seq, &fixture_domain_data(0))
                };
                let peer = src.as_pathname().expect("named client socket");
                server.send_to(&resp, peer).unwrap();
            }
            *done_flag.lock().unwrap() = true;
        });

        let mut client = PtpManagementClient::open(&server_path, &client_path).unwrap();
        let st = client.query_state();
        assert!(st.locked);
        assert_eq!(st.port_state, PortState::Slave);
        assert_eq!(st.domain, 0);

        server_thread.join().unwrap();
        assert!(*done.lock().unwrap());
    }

    #[test]
    fn missing_socket_yields_unlocked() {
        let dir = tempdir().unwrap();
        let server_path = dir.path().join("missing.ro");
        let client_path = dir.path().join("client.pmc");
        let mut client = PtpManagementClient::open(&server_path, &client_path).unwrap();
        let st = client.query_state();
        assert!(!st.locked);
        assert_eq!(st.port_state, PortState::Unknown);
    }

    #[test]
    fn reads_interface_section() {
        let cfg = "\
[global]
clientOnly 1
[enp1s0]
";
        assert_eq!(interface_from_ptp4l_cfg(cfg).as_deref(), Some("enp1s0"));
    }

    #[test]
    fn text_pmc_reply_still_parses() {
        let text = "\
        portState               SLAVE
        domainNumber            0
";
        let st = parse_pmc_reply(text);
        assert!(st.locked);
    }
}
