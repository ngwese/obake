// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Join a multicast group briefly and read one RTP timestamp modulo.

use log::debug;
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::time::Duration;

/// Receive one RTP packet and return `timestamp % framecount`.
///
/// Requires the sender to have `net.loop = true` (or another host sourcing
/// the group) so the join on `iface_ip` observes packets.
pub fn probe_ts_mod(
    group: Ipv4Addr,
    port: u16,
    iface_ip: Ipv4Addr,
    framecount: u32,
) -> Option<u32> {
    if framecount == 0 {
        return None;
    }
    let sock = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port)).ok()?;
    sock.set_read_timeout(Some(Duration::from_millis(500)))
        .ok()?;
    sock.join_multicast_v4(&group, &iface_ip).ok()?;
    let mut buf = [0u8; 2048];
    // Skip a few packets so the stream is past startup.
    let mut ts = 0u32;
    let mut got = false;
    for _ in 0..8 {
        let n = match sock.recv(&mut buf) {
            Ok(n) => n,
            Err(_) => break,
        };
        if n < 12 {
            continue;
        }
        ts = u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]);
        got = true;
    }
    let _ = sock.leave_multicast_v4(&group, &iface_ip);
    if !got {
        return None;
    }
    let rem = ts % framecount;
    debug!("rtp_probe: group={group}:{port} ts={ts} ts%{framecount}={rem}");
    Some(rem)
}

#[cfg(test)]
mod tests {
    #[test]
    fn mod_math_example() {
        // Documented case from bata sniff before alignment.
        assert_eq!(1040451096u32 % 48, 24);
    }
}
