// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Validation of multicast RTP destinations for configured sources.

use std::net::{Ipv4Addr, SocketAddrV4};
use thiserror::Error;

/// Default RTP port when the destination string omits `:port`.
pub const DEFAULT_RTP_PORT: u16 = 5004;
/// Default IP TTL for transmitted multicast when unset in config.
pub const DEFAULT_TTL: u8 = 15;

/// A validated administratively scoped multicast group and even RTP port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MulticastDestination {
    /// Multicast group in `239.0.0.0/8` (not the SAP group).
    pub group: Ipv4Addr,
    /// Even RTP port (RTCP would use port+1; unused this milestone).
    pub port: u16,
}

/// Why a destination string or address pair was rejected.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DestinationError {
    /// Address is not IPv4 multicast in `239.0.0.0/8`.
    #[error("not an IPv4 multicast address")]
    NotMulticast,
    /// `239.255.255.255` is reserved for SAP and must not be used for media.
    #[error("SAP announcement group is reserved")]
    SapGroup,
    /// RTP media ports must be even.
    #[error("RTP port must be even")]
    OddPort,
    /// Destination string could not be parsed as `group` or `group:port`.
    #[error("invalid destination: {0}")]
    Parse(String),
}

impl MulticastDestination {
    /// Parse `group` or `group:port` and apply destination rules.
    pub fn parse(text: &str) -> Result<Self, DestinationError> {
        let (host, port) = match text.rsplit_once(':') {
            Some((h, p)) => {
                let port: u16 = p
                    .parse()
                    .map_err(|_| DestinationError::Parse(text.to_string()))?;
                (h, port)
            }
            None => (text, DEFAULT_RTP_PORT),
        };
        let group: Ipv4Addr = host
            .parse()
            .map_err(|_| DestinationError::Parse(text.to_string()))?;
        Self::new(group, port)
    }

    /// Construct from a group and port after applying the same rules as [`parse`].
    pub fn new(group: Ipv4Addr, port: u16) -> Result<Self, DestinationError> {
        if !group.is_multicast() {
            return Err(DestinationError::NotMulticast);
        }
        // Administratively scoped 239/8 only, per the design.
        if group.octets()[0] != 239 {
            return Err(DestinationError::NotMulticast);
        }
        if group == Ipv4Addr::new(239, 255, 255, 255) {
            return Err(DestinationError::SapGroup);
        }
        if port % 2 != 0 {
            return Err(DestinationError::OddPort);
        }
        Ok(Self { group, port })
    }

    /// Socket address used when loading an RTP sink module.
    pub fn socket_addr(&self) -> SocketAddrV4 {
        SocketAddrV4::new(self.group, self.port)
    }
}

/// Resolve configured TTL, falling back to [`DEFAULT_TTL`].
pub fn resolve_ttl(configured: Option<u8>) -> u8 {
    configured.unwrap_or(DEFAULT_TTL)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_group_and_port() {
        let d = MulticastDestination::parse("239.69.1.10:5004").unwrap();
        assert_eq!(d.group, Ipv4Addr::new(239, 69, 1, 10));
        assert_eq!(d.port, 5004);
    }

    #[test]
    fn bare_group_uses_default_port() {
        let d = MulticastDestination::parse("239.69.1.10").unwrap();
        assert_eq!(d.port, DEFAULT_RTP_PORT);
    }

    #[test]
    fn rejects_sap_group() {
        assert_eq!(
            MulticastDestination::parse("239.255.255.255:5004"),
            Err(DestinationError::SapGroup)
        );
    }

    #[test]
    fn rejects_unicast() {
        assert_eq!(
            MulticastDestination::parse("192.168.1.10:5004"),
            Err(DestinationError::NotMulticast)
        );
    }

    #[test]
    fn rejects_odd_port() {
        assert_eq!(
            MulticastDestination::parse("239.69.1.10:5005"),
            Err(DestinationError::OddPort)
        );
    }

    #[test]
    fn default_ttl_is_15() {
        assert_eq!(resolve_ttl(None), 15);
        assert_eq!(resolve_ttl(Some(8)), 8);
    }
}
