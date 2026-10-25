// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Read `ptp4l` lock state over the read-only management Unix socket.
//!
//! Speaks IEEE 1588 management TLVs in-process (same messages `pmc -u`
//! would send). Never spawns a helper and never adjusts a clock.

use crate::frame_size;
use log::{debug, warn};
use std::fs;
use std::io;
use std::os::unix::net::UnixDatagram;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

const MSG_MANAGEMENT: u8 = 0x0d;
const PTP_VERSION: u8 = 0x12; // minor 1 | major 2, matching linuxptp 4.x
const TLV_MANAGEMENT: u16 = 0x0001;
const MGMT_ACTION_GET: u8 = 0x00;
const MGMT_ACTION_RESPONSE: u8 = 0x02;

const MID_DOMAIN: u16 = 0x2007;
const MID_PORT_DATA_SET: u16 = 0x2004;
const MID_PARENT_DATA_SET: u16 = 0x2002;
/// Offset of `grandmasterIdentity` in a packed IEEE 1588 parentDS.
const PARENT_DS_GMID_OFF: usize = 24;

const MGMT_PREFIX_LEN: usize = 48; // header + targetPortIdentity + hops/action
const TLV_HEADER_LEN: usize = 6; // type + length + id
const GET_REQUEST_LEN: usize = MGMT_PREFIX_LEN + TLV_HEADER_LEN;

/// Latest PTP lock view used by qualify and status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PtpState {
    /// True when [`PortState`] is FOLLOWER (daemon treats that as locked).
    pub locked: bool,
    /// Domain number from the DOMAIN management ID.
    pub domain: u8,
    /// Raw port state from PORT_DATA_SET.
    pub port_state: PortState,
    /// Grandmaster clock identity from PARENT_DATA_SET, if known and non-zero.
    pub gmid: Option<[u8; 8]>,
}

impl PtpState {
    /// Conservative default when the UDS query fails or is unavailable.
    pub fn unlocked() -> Self {
        Self {
            locked: false,
            domain: 0,
            port_state: PortState::Unknown,
            gmid: None,
        }
    }
}

/// Update continuous-lock tracking and return whether the lock-hold has elapsed.
///
/// When `locked` is false, clears `locked_since` and returns false. When true,
/// records the rising edge in `locked_since` (if unset) and returns whether
/// `now - locked_since` is at least `hold_ms` (including when `hold_ms` is 0).
pub fn lock_hold_satisfied(
    locked: bool,
    locked_since: &mut Option<Instant>,
    now: Instant,
    hold_ms: u64,
) -> bool {
    if !locked {
        *locked_since = None;
        return false;
    }
    let since = *locked_since.get_or_insert(now);
    now.duration_since(since).as_millis() >= u128::from(hold_ms)
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
    /// Synchronized to a grandmaster (IEEE portState 9); required for stream up.
    /// Displayed as FOLLOWER; linuxptp management text still says `SLAVE`.
    Follower = 9,
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
            9 => PortState::Follower,
            _ => PortState::Unknown,
        }
    }

    /// Parse a `pmc`-style text token (`FOLLOWER`/`SLAVE`, `LISTENING`, …).
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
            // linuxptp still prints SLAVE; accept FOLLOWER for display parity.
            "SLAVE" | "FOLLOWER" => PortState::Follower,
            _ => PortState::Unknown,
        }
    }

    /// Whether this state counts as locked for stream qualification.
    pub fn is_locked(self) -> bool {
        matches!(self, PortState::Follower)
    }

    /// Stable name for logs and status (`FOLLOWER`, `LISTENING`, …).
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
            PortState::Follower => "FOLLOWER",
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

/// Extract `grandmasterIdentity` from a PARENT_DATA_SET TLV payload.
pub fn parse_parent_data_set(data: &[u8]) -> Result<[u8; 8], String> {
    if data.len() < PARENT_DS_GMID_OFF + 8 {
        return Err("PARENT_DATA_SET data too short".into());
    }
    let mut gmid = [0u8; 8];
    gmid.copy_from_slice(&data[PARENT_DS_GMID_OFF..PARENT_DS_GMID_OFF + 8]);
    if gmid == [0u8; 8] {
        return Err("grandmasterIdentity is all zeros".into());
    }
    Ok(gmid)
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

/// PARENT_DATA_SET-sized fixture with the given grandmaster identity.
pub fn fixture_parent_data_set(gmid: [u8; 8]) -> [u8; 32] {
    let mut data = [0u8; 32];
    data[PARENT_DS_GMID_OFF..PARENT_DS_GMID_OFF + 8].copy_from_slice(&gmid);
    data
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

    /// Query PORT_DATA_SET, DOMAIN, and PARENT_DATA_SET.
    /// On PORT_DATA_SET failure, unlocked with no GMID.
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
        let gmid = match self.transact(MID_PARENT_DATA_SET) {
            Ok(d) => match parse_parent_data_set(&d) {
                Ok(id) => Some(id),
                Err(e) => {
                    debug!("ptp: parse PARENT_DATA_SET failed: {e}");
                    None
                }
            },
            Err(e) => {
                debug!("ptp: PARENT_DATA_SET failed: {e}");
                None
            }
        };
        let state = PtpState {
            locked: port_state.is_locked(),
            domain,
            port_state,
            gmid,
        };
        debug!(
            "ptp: query result locked={} domain={} port_state={} gmid={:?}",
            state.locked,
            state.domain,
            state.port_state.as_str(),
            state.gmid
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

/// Resolve `/dev/ptpN` for a netdev via sysfs (`…/device/ptp/ptpN`).
pub fn phc_device_for_iface(iface: &str) -> Option<PathBuf> {
    let ptp_dir = Path::new("/sys/class/net").join(iface).join("device/ptp");
    let mut entries = fs::read_dir(&ptp_dir).ok()?;
    let ent = entries.next()?.ok()?;
    let name = ent.file_name();
    let name = name.to_str()?;
    if !name.starts_with("ptp") {
        return None;
    }
    Some(PathBuf::from(format!("/dev/{name}")))
}

/// Read PHC time in nanoseconds via `phc_ctl <dev> get`.
pub fn read_phc_time_nsec(iface: &str) -> Option<u64> {
    let dev = phc_device_for_iface(iface)?;
    let output = Command::new("phc_ctl").arg(&dev).arg("get").output().ok()?;
    if !output.status.success() {
        warn!(
            "ptp: phc_ctl {} get failed status={}",
            dev.display(),
            output.status
        );
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    parse_phc_ctl_nsec(&text)
}

/// Parse `phc_ctl … get` stdout (`clock time is SECONDS.FRAC …`).
pub fn parse_phc_ctl_nsec(text: &str) -> Option<u64> {
    for part in text.split_whitespace() {
        if !part.contains('.') {
            continue;
        }
        let Some((secs, frac)) = part.split_once('.') else {
            continue;
        };
        let Ok(secs) = secs.parse::<u64>() else {
            continue;
        };
        let frac_digits: String = frac.chars().take_while(|c| c.is_ascii_digit()).collect();
        if frac_digits.is_empty() {
            continue;
        }
        let mut nsec_str = frac_digits;
        while nsec_str.len() < 9 {
            nsec_str.push('0');
        }
        nsec_str.truncate(9);
        let Ok(nsec) = nsec_str.parse::<u64>() else {
            continue;
        };
        return Some(secs.saturating_mul(1_000_000_000).saturating_add(nsec));
    }
    None
}

/// `sess.ts-offset` that epoch-aligns RTP timestamps for `framecount` at `rate`.
pub fn epoch_align_offset_for_iface(iface: &str, rate: u32, framecount: u32) -> Option<i32> {
    let nsec = read_phc_time_nsec(iface)?;
    let samples = (u128::from(nsec) * u128::from(rate)) / 1_000_000_000u128;
    let offset = frame_size::epoch_align_ts_offset(samples as u64, framecount);
    debug!(
        "ptp: epoch-align iface={iface} rate={rate} framecount={framecount} \
         samples_mod={} offset={offset}",
        (samples as u64) % u64::from(framecount.max(1))
    );
    Some(offset)
}

/// Media-clock sample count from the iface PHC via `clock_gettime`.
pub fn phc_media_samples(iface: &str, rate: u32) -> Option<u64> {
    let dev = phc_device_for_iface(iface)?;
    let cpath = std::ffi::CString::new(dev.to_str()?).ok()?;
    let fd = unsafe { libc::open(cpath.as_ptr(), libc::O_RDONLY) };
    if fd < 0 {
        return None;
    }
    // FD_TO_CLOCKID(fd) = (~fd << 3) | 3
    let clk = ((!fd) << 3) | 3;
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let rc = unsafe { libc::clock_gettime(clk, &mut ts) };
    unsafe { libc::close(fd) };
    if rc != 0 {
        return None;
    }
    let nsec = (ts.tv_sec as u128)
        .saturating_mul(1_000_000_000)
        .saturating_add(ts.tv_nsec as u128);
    Some(((nsec * u128::from(rate)) / 1_000_000_000u128) as u64)
}

/// Busy-wait until `phc_media_samples(iface, rate) % framecount == lead`.
pub fn wait_phc_rem(iface: &str, rate: u32, framecount: u32, lead: u32) -> Result<(), String> {
    if framecount == 0 {
        return Err("framecount 0".into());
    }
    let lead = lead % framecount;
    let start = std::time::Instant::now();
    loop {
        let Some(samples) = phc_media_samples(iface, rate) else {
            return Err(format!("no PHC for {iface}"));
        };
        if (samples % u64::from(framecount)) as u32 == lead {
            debug!(
                "ptp: hit rem={lead} iface={iface} after {:?}",
                start.elapsed()
            );
            return Ok(());
        }
        if start.elapsed() > std::time::Duration::from_secs(2) {
            return Err(format!(
                "timeout waiting for PHC rem={lead} on {iface}"
            ));
        }
        std::thread::sleep(std::time::Duration::from_micros(50));
    }
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
        gmid: None,
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
    fn binary_follower_is_locked() {
        let data = fixture_port_data_set(PortState::Follower);
        let msg = build_response(MID_PORT_DATA_SET, 1, &data);
        let (id, payload) = parse_management_response(&msg).unwrap();
        assert_eq!(id, MID_PORT_DATA_SET);
        let st = parse_port_data_set(payload).unwrap();
        assert_eq!(st, PortState::Follower);
        assert!(st.is_locked());
        assert_eq!(st.as_str(), "FOLLOWER");
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
    fn binary_parent_gmid() {
        let gmid = [0x00, 0x1d, 0xc1, 0xff, 0xfe, 0x12, 0x34, 0x56];
        let data = fixture_parent_data_set(gmid);
        let msg = build_response(MID_PARENT_DATA_SET, 4, &data);
        let (id, payload) = parse_management_response(&msg).unwrap();
        assert_eq!(id, MID_PARENT_DATA_SET);
        assert_eq!(parse_parent_data_set(payload).unwrap(), gmid);
        assert!(parse_parent_data_set(&fixture_parent_data_set([0u8; 8])).is_err());
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

        let gmid = [0x00, 0x1d, 0xc1, 0xff, 0xfe, 0x12, 0x34, 0x56];
        let done = Arc::new(Mutex::new(false));
        let done_flag = Arc::clone(&done);
        let server_thread = thread::spawn(move || {
            let mut buf = [0u8; 512];
            for _ in 0..3 {
                let (n, src) = server.recv_from(&mut buf).unwrap();
                let _ = n;
                let id = u16::from_be_bytes([buf[52], buf[53]]);
                let seq = u16::from_be_bytes([buf[30], buf[31]]);
                let resp = if id == MID_PORT_DATA_SET {
                    build_response(id, seq, &fixture_port_data_set(PortState::Follower))
                } else if id == MID_DOMAIN {
                    build_response(id, seq, &fixture_domain_data(0))
                } else {
                    build_response(id, seq, &fixture_parent_data_set(gmid))
                };
                let peer = src.as_pathname().expect("named client socket");
                server.send_to(&resp, peer).unwrap();
            }
            *done_flag.lock().unwrap() = true;
        });

        let mut client = PtpManagementClient::open(&server_path, &client_path).unwrap();
        let st = client.query_state();
        assert!(st.locked);
        assert_eq!(st.port_state, PortState::Follower);
        assert_eq!(st.domain, 0);
        assert_eq!(st.gmid, Some(gmid));

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
    fn parses_phc_ctl_get_line() {
        let text = "phc_ctl[1087.679]: clock time is 1792891134.886718651 or Sun Oct 25 01:18:54 2026\n";
        let nsec = parse_phc_ctl_nsec(text).unwrap();
        assert_eq!(nsec, 1792891134_886718651);
    }

    #[test]
    fn text_pmc_reply_still_parses_linuxptp_slave_token() {
        let text = "\
        portState               SLAVE
        domainNumber            0
";
        let st = parse_pmc_reply(text);
        assert!(st.locked);
        assert_eq!(st.port_state, PortState::Follower);
        assert_eq!(st.port_state.as_str(), "FOLLOWER");
    }

    #[test]
    fn text_follower_token_parses_as_locked() {
        assert_eq!(PortState::parse("FOLLOWER"), PortState::Follower);
        assert!(PortState::parse("FOLLOWER").is_locked());
    }

    #[test]
    fn lock_hold_not_elapsed_until_duration() {
        let t0 = Instant::now();
        let mut since = None;
        assert!(!lock_hold_satisfied(true, &mut since, t0, 2000));
        assert!(since.is_some());
        assert!(!lock_hold_satisfied(
            true,
            &mut since,
            t0 + Duration::from_millis(1999),
            2000
        ));
        assert!(lock_hold_satisfied(
            true,
            &mut since,
            t0 + Duration::from_millis(2000),
            2000
        ));
    }

    #[test]
    fn lock_hold_zero_satisfied_immediately() {
        let t0 = Instant::now();
        let mut since = None;
        assert!(lock_hold_satisfied(true, &mut since, t0, 0));
    }

    #[test]
    fn unlock_resets_lock_hold_timer() {
        let t0 = Instant::now();
        let mut since = None;
        assert!(!lock_hold_satisfied(true, &mut since, t0, 2000));
        assert!(lock_hold_satisfied(
            true,
            &mut since,
            t0 + Duration::from_millis(2000),
            2000
        ));
        assert!(!lock_hold_satisfied(false, &mut since, t0, 2000));
        assert!(since.is_none());
        let t1 = t0 + Duration::from_secs(10);
        assert!(!lock_hold_satisfied(true, &mut since, t1, 2000));
        assert!(!lock_hold_satisfied(
            true,
            &mut since,
            t1 + Duration::from_millis(1999),
            2000
        ));
        assert!(lock_hold_satisfied(
            true,
            &mut since,
            t1 + Duration::from_millis(2000),
            2000
        ));
    }
}
