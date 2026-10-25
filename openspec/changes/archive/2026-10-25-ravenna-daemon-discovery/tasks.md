# Tasks

## 1. Shared discovery helper (obake-ravenna)

- [x] 1.1 Extract shared helpers for discover_peers → group_by_endpoint
  and for DESCRIBE (encoded path) → parse_sdp into
  `HashMap<(device, stream), StreamSdp>`, and verify unit tests cover
  grouping keys and that DESCRIBE uses `encode_rtsp_path` /
  `describe_path`
- [x] 1.2 Point the `browse` CLI summary path at those helpers (browse
  may still DESCRIBE every visible session) and verify `cargo test`
  still passes for browse/sdp/rtsp tests

## 2. Daemon discovery cache (obake-ravenna)

- [x] 2.1 Add a background refresh that updates a shared discovery map
  on the PTP ifindex: DESCRIBE only configured `(device, stream)` keys
  that are not already held for an established (`SinkUp`) sink; retain
  cache across refreshes; drop entries when the peer disappears, the
  sink goes down / is removed, or ifindex is missing; keep previous map
  on total mDNS failure — and verify unit tests for skip-re-DESCRIBE
  while up, re-DESCRIBE after down, and clear-on-missing-ifindex
- [x] 2.2 Wire the tick to pass the cached map into qualify (no empty
  map, no per-tick DESCRIBE), share “which sinks are up” with the
  refresh task, and verify a matching sink is not forced
  `device-absent` when the cache has SDP

## 3. Docs and headless check (obake-ravenna)

- [x] 3.1 Update `REFERENCE_GUIDE.md` (and README if needed) to state
  that the daemon discovers peer SDP for `[[sink]]` once at establish
  and does not re-DESCRIBE while the sink stays up, and verify the
  guide no longer claims browse results are unused for sinks
- [x] 3.2 Run `cargo test` in `services/obake-ravenna` headlessly and
  verify all tests pass
