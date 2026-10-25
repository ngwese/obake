// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Avahi DNS-SD browse/publish for RAVENNA RTSP services.
//!
//! Production code talks to the host Avahi daemon over the system bus and
//! never opens UDP 5353. Unit tests use [`RecordingAvahi`], which records
//! requests without touching D-Bus.
//!
//! RAVENNA Operating Principles §3.5:
//! - Nodes advertise `_rtsp._tcp` with subtype `_ravenna._sub._rtsp._tcp`
//!   (what peer device lists browse).
//! - Sessions advertise `_rtsp._tcp` with subtype
//!   `_ravenna_session._sub._rtsp._tcp`.

use crate::browse::ResolvedPeer;
use futures_util::StreamExt;
use log::{debug, info, warn};
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::time::Duration;
use zbus::fdo::DBusProxy;
use zbus::message::Type as MessageType;
use zbus::zvariant::OwnedObjectPath;
use zbus::{Connection, MatchRule, MessageStream};

/// Browse type for RAVENNA **nodes** (device lists).
pub const RAVENNA_SERVICE_TYPE: &str = "_ravenna._sub._rtsp._tcp";

/// Browse type for RAVENNA **sessions** (named streams).
pub const RAVENNA_SESSION_SERVICE_TYPE: &str = "_ravenna_session._sub._rtsp._tcp";

/// Primary DNS-SD type registered with Avahi (`AddService`).
const RTSP_SERVICE_TYPE: &str = "_rtsp._tcp";

const AVAHI_DEST: &str = "org.freedesktop.Avahi";
const SERVER_IFACE: &str = "org.freedesktop.Avahi.Server";
const GROUP_IFACE: &str = "org.freedesktop.Avahi.EntryGroup";
/// Avahi `AVAHI_PROTO_INET`. The AES67 NIC is IPv4-only; `PROTO_UNSPEC`
/// also announces an IPv6 SRV that never resolves, and Anubis then leaves
/// RTSP HOST blank (no DESCRIBE, no clock domain).
const PROTO_INET: i32 = 0;

/// Kind of RAVENNA DNS-SD registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PublishKind {
    /// Node RTSP service (`_ravenna` subtype) for device discovery.
    Node,
    /// Named streaming session (`_ravenna_session` subtype).
    Session,
}

impl PublishKind {
    /// Full Avahi subtype string for this registration.
    pub fn subtype(self) -> &'static str {
        match self {
            PublishKind::Node => RAVENNA_SERVICE_TYPE,
            PublishKind::Session => RAVENNA_SESSION_SERVICE_TYPE,
        }
    }

    /// Stable key for tracking entry groups (distinct from DNS-SD name).
    fn group_key(self, name: &str) -> String {
        match self {
            PublishKind::Node => format!("node:{name}"),
            PublishKind::Session => format!("session:{name}"),
        }
    }
}

/// Parameters for registering an RTSP endpoint via Avahi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishRequest {
    /// DNS-SD instance name.
    pub name: String,
    /// Node vs session (selects the RAVENNA subtype).
    pub kind: PublishKind,
    /// RTSP TCP port.
    pub port: u16,
    /// Avahi interface index to publish on (from the `ptp4l` netdev).
    pub interface_index: i32,
    /// DNS-SD TXT records (`key=value` bytes). Empty for node; session has path.
    pub txt: Vec<Vec<u8>>,
}

/// Parameters for browsing peer RAVENNA RTSP services.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowseRequest {
    /// Service type to browse (normally [`RAVENNA_SERVICE_TYPE`]).
    pub service_type: String,
    /// Avahi interface index to browse on.
    pub interface_index: i32,
}

/// Build a node publish call (appears in peer device lists).
pub fn publish_ravenna_node(name: &str, port: u16, interface_index: i32) -> PublishRequest {
    PublishRequest {
        name: name.to_string(),
        kind: PublishKind::Node,
        port,
        interface_index,
        txt: Vec::new(),
    }
}

/// Build a session publish call for a transmitting source stream.
///
/// TXT includes `path=/by-name/<name>` so Merging/Anubis can fill RTSP HOST.
pub fn publish_ravenna_session(name: &str, port: u16, interface_index: i32) -> PublishRequest {
    let path = format!("path=/by-name/{name}");
    PublishRequest {
        name: name.to_string(),
        kind: PublishKind::Session,
        port,
        interface_index,
        txt: vec![path.into_bytes()],
    }
}

/// Build a browse call for RAVENNA RTSP **nodes** on one interface.
pub fn browse_ravenna(interface_index: i32) -> BrowseRequest {
    BrowseRequest {
        service_type: RAVENNA_SERVICE_TYPE.to_string(),
        interface_index,
    }
}

/// Build a browse call for RAVENNA RTSP **sessions** on one interface.
pub fn browse_ravenna_sessions(interface_index: i32) -> BrowseRequest {
    BrowseRequest {
        service_type: RAVENNA_SESSION_SERVICE_TYPE.to_string(),
        interface_index,
    }
}

/// One DNS-SD hit collected during a settle window (before ResolveService).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct BrowseHit {
    kind: PublishKind,
    interface: i32,
    protocol: i32,
    name: String,
    service_type: String,
    domain: String,
}

/// Trait so the daemon can swap a D-Bus client for a test double.
pub trait AvahiClient {
    /// Publish (or replace) a RAVENNA RTSP service record.
    fn publish(&mut self, req: &PublishRequest) -> Result<(), String>;
    /// Withdraw a previously published registration.
    fn unpublish(&mut self, kind: PublishKind, name: &str) -> Result<(), String>;
    /// Start or continue browsing for peer services.
    fn browse(&mut self, req: &BrowseRequest) -> Result<(), String>;
}

/// Records publish/browse calls for tests. Does not open a socket or bus.
#[derive(Debug, Default)]
pub struct RecordingAvahi {
    /// Publish requests received, in order.
    pub published: Vec<PublishRequest>,
    /// `(kind, name)` withdrawn, in order.
    pub unpublished: Vec<(PublishKind, String)>,
    /// Browse requests received, in order.
    pub browsed: Vec<BrowseRequest>,
}

impl AvahiClient for RecordingAvahi {
    fn publish(&mut self, req: &PublishRequest) -> Result<(), String> {
        info!(
            "mdns: publish (recording) kind={:?} name={} subtype={} port={} ifindex={}",
            req.kind,
            req.name,
            req.kind.subtype(),
            req.port,
            req.interface_index
        );
        self.published.push(req.clone());
        Ok(())
    }

    fn unpublish(&mut self, kind: PublishKind, name: &str) -> Result<(), String> {
        info!("mdns: unpublish (recording) kind={kind:?} name={name}");
        self.unpublished.push((kind, name.to_string()));
        Ok(())
    }

    fn browse(&mut self, req: &BrowseRequest) -> Result<(), String> {
        debug!(
            "mdns: browse (recording) type={} ifindex={}",
            req.service_type, req.interface_index
        );
        self.browsed.push(req.clone());
        Ok(())
    }
}

/// Run an Avahi D-Bus future from the sync control-plane tick.
fn block_on_avahi<F, T>(fut: F) -> T
where
    F: std::future::Future<Output = T>,
{
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(fut))
}

/// Avahi client over the system D-Bus (`org.freedesktop.Avahi`).
pub struct SystemAvahi {
    conn: Connection,
    /// Active entry groups keyed by [`PublishKind::group_key`].
    groups: HashMap<String, OwnedObjectPath>,
}

impl SystemAvahi {
    /// Connect to the system bus and confirm Avahi answers.
    pub async fn connect() -> Result<Self, String> {
        let conn = Connection::system()
            .await
            .map_err(|e| format!("system bus: {e}"))?;
        conn.call_method(
            Some(AVAHI_DEST),
            "/",
            Some(SERVER_IFACE),
            "GetVersionString",
            &(),
        )
        .await
        .map_err(|e| format!("avahi GetVersionString: {e}"))?;
        info!("mdns: connected to system Avahi over D-Bus");
        Ok(Self {
            conn,
            groups: HashMap::new(),
        })
    }

    async fn entry_group_new(&self) -> Result<OwnedObjectPath, String> {
        self.conn
            .call_method(
                Some(AVAHI_DEST),
                "/",
                Some(SERVER_IFACE),
                "EntryGroupNew",
                &(),
            )
            .await
            .map_err(|e| format!("EntryGroupNew: {e}"))?
            .body()
            .deserialize()
            .map_err(|e| format!("EntryGroupNew body: {e}"))
    }

    async fn free_group(&self, group: &OwnedObjectPath) -> Result<(), String> {
        self.conn
            .call_method(Some(AVAHI_DEST), group, Some(GROUP_IFACE), "Free", &())
            .await
            .map_err(|e| format!("EntryGroup.Free: {e}"))?;
        Ok(())
    }

    async fn publish_async(&mut self, req: &PublishRequest) -> Result<(), String> {
        if req.interface_index <= 0 {
            return Err(format!(
                "invalid Avahi interface index {} for {}",
                req.interface_index, req.name
            ));
        }

        let key = req.kind.group_key(&req.name);
        if let Some(old) = self.groups.remove(&key) {
            if let Err(e) = self.free_group(&old).await {
                warn!("mdns: freeing previous group for {key}: {e}");
            }
        }

        let group = self.entry_group_new().await?;
        let subtype = req.kind.subtype();
        // Empty TXT still emits the mandatory empty record for node publishes.
        let txt = req.txt.clone();

        let add_result = async {
            self.conn
                .call_method(
                    Some(AVAHI_DEST),
                    &group,
                    Some(GROUP_IFACE),
                    "AddService",
                    &(
                        req.interface_index,
                        PROTO_INET,
                        0u32,
                        req.name.as_str(),
                        RTSP_SERVICE_TYPE,
                        "",
                        "",
                        req.port,
                        txt,
                    ),
                )
                .await
                .map_err(|e| format!("AddService: {e}"))?;

            self.conn
                .call_method(
                    Some(AVAHI_DEST),
                    &group,
                    Some(GROUP_IFACE),
                    "AddServiceSubtype",
                    &(
                        req.interface_index,
                        PROTO_INET,
                        0u32,
                        req.name.as_str(),
                        RTSP_SERVICE_TYPE,
                        "",
                        subtype,
                    ),
                )
                .await
                .map_err(|e| format!("AddServiceSubtype: {e}"))?;

            self.conn
                .call_method(Some(AVAHI_DEST), &group, Some(GROUP_IFACE), "Commit", &())
                .await
                .map_err(|e| format!("Commit: {e}"))?;
            Ok::<(), String>(())
        }
        .await;

        if let Err(e) = add_result {
            let _ = self.free_group(&group).await;
            return Err(e);
        }

        info!(
            "mdns: published kind={:?} name={} type={} subtype={} port={} ifindex={}",
            req.kind, req.name, RTSP_SERVICE_TYPE, subtype, req.port, req.interface_index
        );
        self.groups.insert(key, group);
        Ok(())
    }

    async fn unpublish_async(&mut self, kind: PublishKind, name: &str) -> Result<(), String> {
        let key = kind.group_key(name);
        let Some(group) = self.groups.remove(&key) else {
            debug!("mdns: unpublish {key}: no active group");
            return Ok(());
        };
        self.free_group(&group).await?;
        info!("mdns: unpublished kind={kind:?} name={name}");
        Ok(())
    }

    /// One-shot browse of RAVENNA nodes and sessions on `ifindex`.
    ///
    /// Starts ServiceBrowsers for both subtypes, collects `ItemNew` for
    /// `settle`, resolves each hit to IPv4+port+TXT, then frees the browsers.
    /// Returns `(devices, sessions)`.
    pub async fn discover_peers(
        &self,
        ifindex: i32,
        settle: Duration,
    ) -> Result<(Vec<ResolvedPeer>, Vec<ResolvedPeer>), String> {
        if ifindex <= 0 {
            return Err(format!("invalid Avahi interface index {ifindex}"));
        }
        let hits = self.collect_browse_hits(ifindex, settle).await?;
        let mut devices = Vec::new();
        let mut sessions = Vec::new();
        for hit in hits {
            match self.resolve_hit(&hit).await {
                Ok(peer) => match hit.kind {
                    PublishKind::Node => devices.push(peer),
                    PublishKind::Session => sessions.push(peer),
                },
                Err(e) => warn!(
                    "mdns: resolve kind={:?} name={} failed: {e}",
                    hit.kind, hit.name
                ),
            }
        }
        devices.sort_by(|a, b| a.name.cmp(&b.name));
        sessions.sort_by(|a, b| a.name.cmp(&b.name));
        debug!(
            "mdns: discover devices={} sessions={} ifindex={ifindex}",
            devices.len(),
            sessions.len()
        );
        Ok((devices, sessions))
    }

    async fn service_browser_new(
        &self,
        ifindex: i32,
        service_type: &str,
    ) -> Result<OwnedObjectPath, String> {
        self.conn
            .call_method(
                Some(AVAHI_DEST),
                "/",
                Some(SERVER_IFACE),
                "ServiceBrowserNew",
                &(ifindex, PROTO_INET, service_type, "", 0u32),
            )
            .await
            .map_err(|e| format!("ServiceBrowserNew({service_type}): {e}"))?
            .body()
            .deserialize()
            .map_err(|e| format!("ServiceBrowserNew body: {e}"))
    }

    async fn free_browser(&self, path: &OwnedObjectPath) -> Result<(), String> {
        self.conn
            .call_method(
                Some(AVAHI_DEST),
                path,
                Some("org.freedesktop.Avahi.ServiceBrowser"),
                "Free",
                &(),
            )
            .await
            .map_err(|e| format!("ServiceBrowser.Free: {e}"))?;
        Ok(())
    }

    async fn collect_browse_hits(
        &self,
        ifindex: i32,
        settle: Duration,
    ) -> Result<Vec<BrowseHit>, String> {
        let node_path = self
            .service_browser_new(ifindex, RAVENNA_SERVICE_TYPE)
            .await?;
        let session_path = self
            .service_browser_new(ifindex, RAVENNA_SESSION_SERVICE_TYPE)
            .await?;
        debug!(
            "mdns: browsers node={} session={} settle={}ms",
            node_path,
            session_path,
            settle.as_millis()
        );

        let dbus = DBusProxy::new(&self.conn)
            .await
            .map_err(|e| format!("DBusProxy: {e}"))?;
        let rule = MatchRule::builder()
            .msg_type(MessageType::Signal)
            .interface("org.freedesktop.Avahi.ServiceBrowser")
            .map_err(|e| format!("match interface: {e}"))?
            .member("ItemNew")
            .map_err(|e| format!("match member: {e}"))?
            .build();
        dbus.add_match_rule(rule.clone())
            .await
            .map_err(|e| format!("AddMatch: {e}"))?;

        let mut stream = MessageStream::from(&self.conn);
        let mut hits: HashMap<(PublishKind, String), BrowseHit> = HashMap::new();
        let deadline = tokio::time::Instant::now() + settle;

        loop {
            let now = tokio::time::Instant::now();
            if now >= deadline {
                break;
            }
            let remaining = deadline - now;
            match tokio::time::timeout(remaining, stream.next()).await {
                Ok(Some(Ok(msg))) => {
                    if msg.message_type() != MessageType::Signal {
                        continue;
                    }
                    let header = msg.header();
                    let member = header.member().map(|m| m.as_str().to_string());
                    if member.as_deref() != Some("ItemNew") {
                        continue;
                    }
                    let path = match header.path() {
                        Some(p) => p.as_str().to_string(),
                        None => continue,
                    };
                    let kind = if path == node_path.as_str() {
                        PublishKind::Node
                    } else if path == session_path.as_str() {
                        PublishKind::Session
                    } else {
                        continue;
                    };
                    let body: (i32, i32, String, String, String, u32) = match msg.body().deserialize()
                    {
                        Ok(b) => b,
                        Err(e) => {
                            debug!("mdns: ItemNew body: {e}");
                            continue;
                        }
                    };
                    let (iface, protocol, name, service_type, domain, _flags) = body;
                    hits.insert(
                        (kind, name.clone()),
                        BrowseHit {
                            kind,
                            interface: iface,
                            protocol,
                            name,
                            service_type,
                            domain,
                        },
                    );
                }
                Ok(Some(Err(e))) => {
                    let _ = dbus.remove_match_rule(rule).await;
                    let _ = self.free_browser(&node_path).await;
                    let _ = self.free_browser(&session_path).await;
                    return Err(format!("mdns signal stream: {e}"));
                }
                Ok(None) => break,
                Err(_) => break,
            }
        }

        let _ = dbus.remove_match_rule(rule).await;
        let _ = self.free_browser(&node_path).await;
        let _ = self.free_browser(&session_path).await;
        Ok(hits.into_values().collect())
    }

    async fn resolve_hit(&self, hit: &BrowseHit) -> Result<ResolvedPeer, String> {
        // ResolveService → interface, protocol, name, type, domain, host,
        // aprotocol, address, port, txt, flags
        let reply = self
            .conn
            .call_method(
                Some(AVAHI_DEST),
                "/",
                Some(SERVER_IFACE),
                "ResolveService",
                &(
                    hit.interface,
                    hit.protocol,
                    hit.name.as_str(),
                    hit.service_type.as_str(),
                    hit.domain.as_str(),
                    PROTO_INET,
                    0u32,
                ),
            )
            .await
            .map_err(|e| format!("ResolveService({}): {e}", hit.name))?;
        let body: (
            i32,
            i32,
            String,
            String,
            String,
            String,
            i32,
            String,
            u16,
            Vec<Vec<u8>>,
            u32,
        ) = reply
            .body()
            .deserialize()
            .map_err(|e| format!("ResolveService body: {e}"))?;
        let (_iface, _proto, name, _ty, _domain, _host, _aproto, address, port, txt, _flags) =
            body;
        let address: Ipv4Addr = address
            .parse()
            .map_err(|_| format!("ResolveService address not IPv4: {address}"))?;
        debug!(
            "mdns: resolved kind={:?} name={name} {address}:{port} txt={}",
            hit.kind,
            txt.len()
        );
        Ok(ResolvedPeer {
            name,
            address,
            port,
            txt,
        })
    }
}

impl AvahiClient for SystemAvahi {
    fn publish(&mut self, req: &PublishRequest) -> Result<(), String> {
        block_on_avahi(self.publish_async(req))
    }

    fn unpublish(&mut self, kind: PublishKind, name: &str) -> Result<(), String> {
        block_on_avahi(self.unpublish_async(kind, name))
    }

    fn browse(&mut self, req: &BrowseRequest) -> Result<(), String> {
        // Peer discovery via ServiceBrowser signals is not wired yet; sinks
        // that need a browsed device name stay held down.
        debug!(
            "mdns: browse not yet implemented type={} ifindex={}",
            req.service_type, req.interface_index
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_publish_uses_ravenna_subtype() {
        let mut client = RecordingAvahi::default();
        let req = publish_ravenna_node("bata", 8554, 3);
        client.publish(&req).unwrap();
        assert_eq!(req.kind, PublishKind::Node);
        assert_eq!(req.kind.subtype(), RAVENNA_SERVICE_TYPE);
        assert_eq!(client.published[0].interface_index, 3);
        assert_eq!(client.published[0].port, 8554);
        assert!(client.browsed.is_empty());
    }

    #[test]
    fn session_publish_uses_ravenna_session_subtype() {
        // Live Anubis / Merging sessions on the AES67 segment advertise an
        // empty TXT. We still publish path=/by-name/<stream> so peers that
        // read it can fill RTSP HOST (blank HOST was the board symptom).
        let req = publish_ravenna_session("obake-out-1", 8554, 3);
        assert_eq!(req.kind, PublishKind::Session);
        assert_eq!(req.kind.subtype(), RAVENNA_SESSION_SERVICE_TYPE);
        assert_eq!(
            req.txt,
            vec![b"path=/by-name/obake-out-1".to_vec()]
        );
    }

    #[test]
    fn node_publish_has_empty_txt() {
        let req = publish_ravenna_node("bata", 8554, 3);
        assert!(req.txt.is_empty());
    }

    #[test]
    fn unpublish_is_recorded() {
        let mut client = RecordingAvahi::default();
        client.unpublish(PublishKind::Session, "obake-out-1").unwrap();
        assert_eq!(
            client.unpublished,
            vec![(PublishKind::Session, "obake-out-1".to_string())]
        );
    }

    #[test]
    fn browse_uses_ravenna_type() {
        let req = browse_ravenna(2);
        assert_eq!(req.service_type, RAVENNA_SERVICE_TYPE);
        assert_eq!(req.interface_index, 2);
    }

    #[test]
    fn browse_sessions_uses_ravenna_session_type() {
        let req = browse_ravenna_sessions(2);
        assert_eq!(req.service_type, RAVENNA_SESSION_SERVICE_TYPE);
        assert_eq!(req.interface_index, 2);
    }

    #[test]
    fn recording_avahi_records_both_browse_requests() {
        let mut client = RecordingAvahi::default();
        client.browse(&browse_ravenna(3)).unwrap();
        client.browse(&browse_ravenna_sessions(3)).unwrap();
        assert_eq!(client.browsed.len(), 2);
        assert_eq!(client.browsed[0].service_type, RAVENNA_SERVICE_TYPE);
        assert_eq!(
            client.browsed[1].service_type,
            RAVENNA_SESSION_SERVICE_TYPE
        );
        assert_eq!(client.browsed[0].interface_index, 3);
        assert_eq!(client.browsed[1].interface_index, 3);
    }
}
