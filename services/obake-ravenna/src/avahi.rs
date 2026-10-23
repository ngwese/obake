// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Avahi DNS-SD browse/publish requests for RAVENNA RTSP services.
//!
//! The real process talks to Avahi over the system bus. Unit tests exercise
//! the request builder, which never opens UDP 5353.

/// DNS-SD type string for RAVENNA RTSP services.
pub const RAVENNA_SERVICE_TYPE: &str = "_ravenna._sub._rtsp._tcp";

/// Parameters for registering this host's RTSP endpoint via Avahi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishRequest {
    /// DNS-SD instance name.
    pub name: String,
    /// Service type (normally [`RAVENNA_SERVICE_TYPE`]).
    pub service_type: String,
    /// RTSP TCP port.
    pub port: u16,
    /// Avahi interface index to publish on.
    pub interface_index: u32,
}

/// Parameters for browsing peer RAVENNA RTSP services.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowseRequest {
    /// Service type to browse (normally [`RAVENNA_SERVICE_TYPE`]).
    pub service_type: String,
    /// Avahi interface index to browse on.
    pub interface_index: u32,
}

/// Build a publish call for the host Avahi daemon on one interface.
pub fn publish_ravenna(name: &str, port: u16, interface_index: u32) -> PublishRequest {
    PublishRequest {
        name: name.to_string(),
        service_type: RAVENNA_SERVICE_TYPE.to_string(),
        port,
        interface_index,
    }
}

/// Build a browse call for RAVENNA RTSP peers on one interface.
pub fn browse_ravenna(interface_index: u32) -> BrowseRequest {
    BrowseRequest {
        service_type: RAVENNA_SERVICE_TYPE.to_string(),
        interface_index,
    }
}

/// Trait so the daemon can swap a D-Bus client for a test double.
pub trait AvahiClient {
    /// Publish (or refresh) a RAVENNA RTSP service record.
    fn publish(&mut self, req: &PublishRequest) -> Result<(), String>;
    /// Start or continue browsing for peer services.
    fn browse(&mut self, req: &BrowseRequest) -> Result<(), String>;
}

/// Records publish/browse calls for tests. Does not open a socket.
#[derive(Debug, Default)]
pub struct RecordingAvahi {
    /// Publish requests received, in order.
    pub published: Vec<PublishRequest>,
    /// Browse requests received, in order.
    pub browsed: Vec<BrowseRequest>,
}

impl AvahiClient for RecordingAvahi {
    fn publish(&mut self, req: &PublishRequest) -> Result<(), String> {
        self.published.push(req.clone());
        Ok(())
    }

    fn browse(&mut self, req: &BrowseRequest) -> Result<(), String> {
        self.browsed.push(req.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_uses_ravenna_type_and_interface() {
        let mut client = RecordingAvahi::default();
        let req = publish_ravenna("obake", 8554, 3);
        client.publish(&req).unwrap();
        assert_eq!(client.published[0].service_type, RAVENNA_SERVICE_TYPE);
        assert_eq!(client.published[0].interface_index, 3);
        assert_eq!(client.published[0].port, 8554);
        // No UDP 5353: this client never opens a socket.
        assert!(client.browsed.is_empty());
    }

    #[test]
    fn browse_uses_ravenna_type() {
        let req = browse_ravenna(2);
        assert_eq!(req.service_type, RAVENNA_SERVICE_TYPE);
        assert_eq!(req.interface_index, 2);
    }
}
