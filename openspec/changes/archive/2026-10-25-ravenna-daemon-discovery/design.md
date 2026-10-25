# Design

## Context

See proposal.md. Qualify already takes
`discovered: HashMap<(device, stream), StreamSdp>` and maps
`device-absent` on miss. The tick still assigns an empty map.
`SystemAvahi::discover_peers`, `describe_path`, `encode_rtsp_path`,
`describe_sdp`, `group_by_endpoint`, and `parse_sdp` (including
framecount ranges) already exist for the `browse` CLI.

## Goals / Non-Goals

**Goals:**

- Fill the discovery map so configured sinks can qualify.
- Share one discovery/DESCRIBE path with `browse` (encoding included).
- Avoid blocking the 2s control tick on a multi-second mDNS settle.
- Once a sink is up, do not re-lookup or re-DESCRIBE that session every
  tick or refresh.

**Non-Goals:**

- Changing sink config keys or qualify rules beyond using a live map.
- SAP discovery, unicast SDP, or non-RAVENNA peers.
- Making `browse` talk to the daemon (it stays standalone).
- Continuous SDP polling for live sinks.

## Decisions

1. **Background refresh, tick reads a snapshot.** A tokio task (or
   equivalent) periodically runs mDNS discover → group, then selective
   DESCRIBE, and stores
   `Arc<Mutex<HashMap<(String, String), StreamSdp>>>`. Each tick locks
   briefly and passes that map into qualify. Alternative: oneshot settle
   inside `tick`. Rejected: settle of 2–3s would stall the control loop.

2. **Reuse `SystemAvahi::discover_peers` + shared helper.** Extract
   helpers for discover/group and for DESCRIBE+parse used by the
   background task and by `browse`. Keys: device DNS-SD instance name +
   session instance name after `group_by_endpoint`. Orphan sessions (no
   device) are not entered under a device key; they cannot satisfy
   `[[sink]].device`.

3. **DESCRIBE only unsettled sink keys (establish-once).** Maintain the
   SDP cache across refreshes. On each refresh:
   - Learn which `(device, stream)` pairs are visible via mDNS.
   - **DESCRIBE** only pairs that are needed by a configured sink and
     that do **not** already have a cache entry used by an established
     (`SinkUp`) stream — i.e. first establish, or re-establish after
     down/clear.
   - **Do not** re-DESCRIBE keys whose sink is currently up.
   - Drop a cache entry when the peer advertisement disappears, when
     the matching sink goes down / is removed from config, or when
     ifindex is missing (clear map). That allows a later DESCRIBE.
   - On total mDNS refresh failure, keep the previous cache (established
     sinks keep working).

4. **Shared RTSP client.** Daemon and `browse` both call `describe_sdp`
   (already encodes via `encode_rtsp_path`). No second client. The
   one-shot `browse` command may still DESCRIBE every visible session;
   only the daemon control path is establish-once.

5. **Avahi connection.** Background task owns a `SystemAvahi`
   connection for discover; the tick continues to use the existing
   publish client. Two connections to the system bus are acceptable.

6. **Refresh interval / settle.** Default settle ~2s, period ~5s
   (tunable constants, not config keys in this change). Log when cache
   keys are added or removed; failures at warn.

7. **Interface absence.** When PTP ifindex is missing, clear the
   discovery map (sinks stay `device-absent` / `interface-missing` per
   existing gates).

8. **Feeding “which sinks are up” into refresh.** The background task
   needs the set of `(device, stream)` that are currently `SinkUp` (or
   the set of cache keys that must not be re-DESCRIBEd). Share that via
   the same mutex bundle or a small `Arc<Mutex<…>>` updated each tick
   from qualify results / stream state.

## Risks / Trade-offs

- **[Risk] Stale SDP if a peer changes parameters without withdrawing
  mDNS** → Mitigation: accept until sink drops or advertisement
  disappears; operator can reload/restart. No per-tick DESCRIBE by
  design.
- **[Risk] DESCRIBE load while many sinks are still down** → Mitigation:
  only configured unsettled keys; small studio scale.
- **[Risk] Dual Avahi clients** → Mitigation: same pattern as today;
  document in guide.

## Migration Plan

Ship the binary. Existing `[[sink]]` rows start working when peers are
visible; no config migration. Restart picks up the background task
(reload alone is insufficient for new tasks spawned at startup).

## Open Questions

None; refresh timing can be tuned during implementation without
changing requirements.
