// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Shared RAVENNA peer discovery: mDNS group → DESCRIBE → SDP cache.
//!
//! The one-shot `browse` CLI DESCRIBEs every visible session. The daemon
//! refresh path DESCRIBEs only configured keys missing from the cache and
//! never re-DESCRIBEs an entry that is already cached (establish-once).

use crate::avahi::SystemAvahi;
use crate::browse::{
    describe_path, group_by_endpoint, ResolvedPeer, SessionSummary,
};
use crate::rtsp::{describe_sdp, DescribeClientError};
use crate::sdp::{self, StreamSdp};
use log::{debug, info, warn};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// `(device instance name, session instance name)`.
pub type SessionKey = (String, String);

/// Per-DESCRIBE timeout.
pub const DESCRIBE_TIMEOUT: Duration = Duration::from_secs(3);

/// Shared SDP map consumed by the control-loop tick.
pub type SharedDiscoveryMap = Arc<Mutex<HashMap<SessionKey, StreamSdp>>>;

/// Configured sink identities the refresh task should try to resolve.
pub type SharedWantedKeys = Arc<Mutex<HashSet<SessionKey>>>;

/// Optional ifindex the refresh task should browse (`None` → clear cache).
pub type SharedIfindex = Arc<Mutex<Option<i32>>>;

/// Live settle/period from `[discovery]` (updated on config reload).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiscoveryTiming {
    /// mDNS settle window.
    pub settle: Duration,
    /// Pause between refreshes.
    pub period: Duration,
}

impl DiscoveryTiming {
    /// Build from config milliseconds (zero period becomes 1 ms to avoid spin).
    pub fn from_ms(settle_ms: u64, period_ms: u64) -> Self {
        Self {
            settle: Duration::from_millis(settle_ms),
            period: Duration::from_millis(period_ms.max(1)),
        }
    }
}

/// Shared discovery timing for the background task.
pub type SharedDiscoveryTiming = Arc<Mutex<DiscoveryTiming>>;

/// Create an empty shared discovery map.
pub fn new_shared_map() -> SharedDiscoveryMap {
    Arc::new(Mutex::new(HashMap::new()))
}

/// Create an empty set of wanted `(device, stream)` keys.
pub fn new_shared_wanted() -> SharedWantedKeys {
    Arc::new(Mutex::new(HashSet::new()))
}

/// Create a shared optional PTP ifindex.
pub fn new_shared_ifindex() -> SharedIfindex {
    Arc::new(Mutex::new(None))
}

/// Create shared timing from config defaults.
pub fn new_shared_timing(timing: DiscoveryTiming) -> SharedDiscoveryTiming {
    Arc::new(Mutex::new(timing))
}

/// Map each session under a device to `(device_name, session_name) → peer`.
///
/// Orphan sessions (no matching device endpoint) are omitted.
pub fn visible_session_peers(
    devices: &[ResolvedPeer],
    sessions: &[ResolvedPeer],
) -> HashMap<SessionKey, ResolvedPeer> {
    let (matched, _orphans) = group_by_endpoint(devices, sessions);
    let mut out = HashMap::new();
    for (device, sess) in matched {
        for s in sess {
            out.insert((device.name.clone(), s.name.clone()), s);
        }
    }
    out
}

/// Keys that should be DESCRIBEd: wanted, visible, and not yet cached.
pub fn keys_needing_describe(
    wanted: &HashSet<SessionKey>,
    visible: &HashMap<SessionKey, ResolvedPeer>,
    cache: &HashMap<SessionKey, StreamSdp>,
) -> Vec<SessionKey> {
    let mut keys: Vec<SessionKey> = wanted
        .iter()
        .filter(|k| visible.contains_key(*k) && !cache.contains_key(*k))
        .cloned()
        .collect();
    keys.sort();
    keys
}

/// Apply a refresh result to the SDP cache (pure; for unit tests).
///
/// - Evicts entries that are no longer visible or no longer wanted.
/// - Inserts newly DESCRIBEd SDP bodies.
/// - Leaves existing entries untouched (no re-DESCRIBE while cached).
pub fn merge_discovery_cache(
    cache: &mut HashMap<SessionKey, StreamSdp>,
    wanted: &HashSet<SessionKey>,
    visible: &HashSet<SessionKey>,
    newly_described: HashMap<SessionKey, StreamSdp>,
) {
    cache.retain(|k, _| wanted.contains(k) && visible.contains(k));
    for (k, sdp) in newly_described {
        cache.insert(k, sdp);
    }
}

/// DESCRIBE one resolved session peer and parse SDP.
pub async fn describe_peer_sdp(
    peer: &ResolvedPeer,
) -> Result<StreamSdp, DescribeClientError> {
    let path = describe_path(&peer.name, &peer.txt);
    let body = describe_sdp(peer.address, peer.port, &path, DESCRIBE_TIMEOUT).await?;
    sdp::parse_sdp(&body).map_err(|e| {
        debug!("discovery: sdp-invalid name={}: {e}", peer.name);
        DescribeClientError::InvalidResponse
    })
}

/// DESCRIBE every peer (browse CLI). Returns per-session-name summaries and
/// optional raw bodies when `keep_raw` is true.
pub async fn describe_all_peers(
    peers: &[ResolvedPeer],
    keep_raw: bool,
) -> (
    HashMap<String, SessionSummary>,
    HashMap<String, String>,
) {
    let mut summaries = HashMap::new();
    let mut raw_bodies = HashMap::new();
    for peer in peers {
        let path = describe_path(&peer.name, &peer.txt);
        let summary = match describe_sdp(peer.address, peer.port, &path, DESCRIBE_TIMEOUT).await
        {
            Ok(body) => {
                if keep_raw {
                    raw_bodies.insert(peer.name.clone(), body.clone());
                }
                match sdp::parse_sdp(&body) {
                    Ok(parsed) => SessionSummary::Ok(parsed),
                    Err(e) => {
                        debug!("discovery: sdp-invalid name={}: {e}", peer.name);
                        SessionSummary::Failed("sdp-invalid")
                    }
                }
            }
            Err(DescribeClientError::Timeout) => {
                debug!("discovery: describe timeout name={}", peer.name);
                SessionSummary::Failed("describe-failed")
            }
            Err(e) => {
                debug!("discovery: describe failed name={}: {e}", peer.name);
                SessionSummary::Failed("describe-failed")
            }
        };
        summaries.insert(peer.name.clone(), summary);
    }
    (summaries, raw_bodies)
}

/// One background refresh iteration (mDNS + selective DESCRIBE).
pub async fn refresh_discovery_once(
    avahi: &SystemAvahi,
    ifindex: Option<i32>,
    settle: Duration,
    wanted: &HashSet<SessionKey>,
    cache: &mut HashMap<SessionKey, StreamSdp>,
) -> Result<(), String> {
    let Some(ifindex) = ifindex.filter(|i| *i > 0) else {
        if !cache.is_empty() {
            info!("discovery: clearing cache (no PTP ifindex)");
            cache.clear();
        }
        return Ok(());
    };

    let (devices, sessions) = match avahi.discover_peers(ifindex, settle).await {
        Ok(v) => v,
        Err(e) => {
            // Keep previous cache on total mDNS failure.
            return Err(e);
        }
    };

    let visible_peers = visible_session_peers(&devices, &sessions);
    let visible_keys: HashSet<SessionKey> = visible_peers.keys().cloned().collect();
    let need = keys_needing_describe(wanted, &visible_peers, cache);

    let mut newly = HashMap::new();
    for key in &need {
        let Some(peer) = visible_peers.get(key) else {
            continue;
        };
        match describe_peer_sdp(peer).await {
            Ok(sdp) => {
                info!(
                    "discovery: described device={} stream={} {}/{}/{}",
                    key.0,
                    key.1,
                    sdp.format.as_str(),
                    sdp.rate,
                    sdp.channels
                );
                newly.insert(key.clone(), sdp);
            }
            Err(e) => {
                warn!(
                    "discovery: DESCRIBE failed device={} stream={}: {e}",
                    key.0, key.1
                );
            }
        }
    }

    let before = cache.len();
    merge_discovery_cache(cache, wanted, &visible_keys, newly);
    if cache.len() != before {
        info!(
            "discovery: cache size {} -> {} (wanted={} visible={})",
            before,
            cache.len(),
            wanted.len(),
            visible_keys.len()
        );
    }
    Ok(())
}

/// Spawn the daemon discovery refresh loop.
pub fn spawn_discovery_task(
    map: SharedDiscoveryMap,
    wanted: SharedWantedKeys,
    ifindex: SharedIfindex,
    timing: SharedDiscoveryTiming,
) {
    tokio::spawn(async move {
        let avahi = match SystemAvahi::connect().await {
            Ok(c) => c,
            Err(e) => {
                warn!("discovery: Avahi unavailable ({e}); discovery idle");
                return;
            }
        };
        let mut last_logged = None;
        loop {
            let t = *timing.lock().unwrap();
            if last_logged != Some(t) {
                info!(
                    "discovery: background refresh settle={}ms period={}ms",
                    t.settle.as_millis(),
                    t.period.as_millis()
                );
                last_logged = Some(t);
            }
            let idx = *ifindex.lock().unwrap();
            let want = wanted.lock().unwrap().clone();
            let mut cache = map.lock().unwrap().clone();
            match refresh_discovery_once(&avahi, idx, t.settle, &want, &mut cache).await {
                Ok(()) => {
                    *map.lock().unwrap() = cache;
                }
                Err(e) => {
                    warn!("discovery: refresh failed (keeping cache): {e}");
                }
            }
            tokio::time::sleep(t.period).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Format;
    use std::net::Ipv4Addr;

    fn peer(name: &str, ip: [u8; 4], port: u16) -> ResolvedPeer {
        ResolvedPeer {
            name: name.into(),
            address: Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]),
            port,
            txt: vec![],
        }
    }

    fn sdp() -> StreamSdp {
        StreamSdp {
            group: Ipv4Addr::new(239, 1, 0, 1),
            port: 5004,
            rate: 48_000,
            channels: 2,
            format: Format::L24,
            frame_size: 48,
            domain: 0,
            payload_type: 98,
        }
    }

    #[test]
    fn visible_maps_device_and_session_names() {
        let devices = vec![peer("Anubis_610267", [192, 168, 60, 70], 80)];
        let sessions = vec![peer(
            "Anubis_610267_Combo 1/2",
            [192, 168, 60, 70],
            80,
        )];
        let vis = visible_session_peers(&devices, &sessions);
        assert!(vis.contains_key(&(
            "Anubis_610267".into(),
            "Anubis_610267_Combo 1/2".into()
        )));
    }

    #[test]
    fn visible_skips_orphan_sessions() {
        let devices = vec![peer("bata", [192, 168, 60, 50], 8554)];
        let sessions = vec![peer("Lone", [192, 168, 60, 99], 8554)];
        let vis = visible_session_peers(&devices, &sessions);
        assert!(vis.is_empty());
    }

    #[test]
    fn keys_needing_describe_skips_cached() {
        let key = ("Anubis".into(), "Combo".into());
        let mut wanted = HashSet::new();
        wanted.insert(key.clone());
        let mut visible = HashMap::new();
        visible.insert(key.clone(), peer("Combo", [1, 2, 3, 4], 80));
        let mut cache = HashMap::new();
        cache.insert(key.clone(), sdp());
        assert!(keys_needing_describe(&wanted, &visible, &cache).is_empty());
        cache.clear();
        assert_eq!(
            keys_needing_describe(&wanted, &visible, &cache),
            vec![key]
        );
    }

    #[test]
    fn merge_evicts_invisible_and_unwanted_keeps_cached() {
        let keep = ("A".into(), "s1".into());
        let gone = ("A".into(), "s2".into());
        let mut cache = HashMap::new();
        cache.insert(keep.clone(), sdp());
        cache.insert(gone.clone(), sdp());
        let mut wanted = HashSet::new();
        wanted.insert(keep.clone());
        let mut visible = HashSet::new();
        visible.insert(keep.clone());
        // No newly described — existing keep stays, gone evicted.
        merge_discovery_cache(&mut cache, &wanted, &visible, HashMap::new());
        assert!(cache.contains_key(&keep));
        assert!(!cache.contains_key(&gone));
    }

    #[test]
    fn merge_inserts_new_without_replacing_unrelated() {
        let a = ("A".into(), "s1".into());
        let b = ("B".into(), "s2".into());
        let mut cache = HashMap::new();
        cache.insert(a.clone(), sdp());
        let mut wanted = HashSet::new();
        wanted.insert(a.clone());
        wanted.insert(b.clone());
        let mut visible = HashSet::new();
        visible.insert(a.clone());
        visible.insert(b.clone());
        let mut newly = HashMap::new();
        let mut other = sdp();
        other.port = 5006;
        newly.insert(b.clone(), other.clone());
        merge_discovery_cache(&mut cache, &wanted, &visible, newly);
        assert_eq!(cache.get(&a).unwrap().port, 5004);
        assert_eq!(cache.get(&b).unwrap().port, 5006);
    }

    #[test]
    fn clear_when_nothing_wanted_or_visible() {
        let mut cache = HashMap::new();
        cache.insert(("A".into(), "s".into()), sdp());
        merge_discovery_cache(
            &mut cache,
            &HashSet::new(),
            &HashSet::new(),
            HashMap::new(),
        );
        assert!(cache.is_empty());
    }
}
