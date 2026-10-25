# Design

## Context

See proposal.md. `obake-ravenna` already publishes node and session
DNS-SD records via Avahi, serves DESCRIBE, and parses SDP into
`StreamSdp`. `SystemAvahi::browse` is a no-op stub used only by the
daemon tick; the empty discovery map is out of scope here. CLI today is
`run` | `status` (`status` talks to a Unix socket on the daemon).

## Goals / Non-Goals

**Goals:**

- One-shot `browse` that scans the PTP NIC, groups sessions under
  devices, DESCRIBEs each session, and prints a human report.
- Reuse `parse_sdp` / `StreamSdp` and the same PTP-interface resolution
  pattern as `run` (`--ptp4l-config` → ifindex).
- Keep discovery helpers usable later by the daemon without wiring them
  into qualify in this change.

**Non-Goals:**

- Feeding browse results into sink qualify or changing `device-absent`.
- Implementing the daemon's long-lived ServiceBrowser for the control
  loop (beyond whatever shared helpers `browse` needs).
- JSON/machine output, filtering flags, or interactive TUI.
- Changing publish, RTSP server, PipeWire, or config schema.

## Decisions

1. **Standalone process, not a daemon RPC.** `browse` opens the system
   Avahi bus and peer RTSP itself. Alternative: ask the running daemon
   for a cache. Rejected: the daemon has no cache yet, and a diagnostic
   must work when the daemon is down or sinks are held.

2. **Browse both subtypes, then group by resolved endpoint.** Collect
   `_ravenna._sub._rtsp._tcp` (devices) and
   `_ravenna_session._sub._rtsp._tcp` (sessions), resolve each to
   host:port (+ TXT), and attach sessions to devices that share the
   same resolved IPv4 and port. Sessions with no matching device appear
   under a synthetic "unassigned" section (or equivalent) so orphans
   stay visible. Alternative: DESCRIBE only sessions and ignore device
   records. Rejected: operators need device names for `[[sink]].device`.

3. **Settle window then snapshot.** Avahi ServiceBrowser is event-driven.
   Wait for `--timeout` (default **3s**) after starting both browsers,
   then stop collecting and resolve/DESCRIBE what arrived. Alternative:
   exit on first AllForNow. Rejected as too racy on a quiet segment;
   a fixed settle is predictable for a one-shot tool.

4. **DESCRIBE path.** Prefer TXT `path=` (strip a leading `path=` if the
   record is stored as a full `key=value` byte string). Else
   `/by-name/<DNS-SD instance name>`. Use a short RTSP client (OPTIONS
   optional; DESCRIBE with `Accept: application/sdp`) over TCP to the
   resolved address. Parse with existing `parse_sdp`.

5. **Interface scoping.** Same defaults as `run`:
   `--ptp4l-config` → interface name → ifindex. Browse only on that
   Avahi interface index (`PROTO_INET`). If the interface is missing,
   print an empty/error report and exit non-zero without scanning all
   interfaces.

6. **Report shape.** Human text aligned with `status` tone:

   ```
   device  <name>  <ip>:<port>
     session  <name>  <fmt> <rate>/<ch>  frames=<n>  <group>:<port>
   ```

   On DESCRIBE/SDP failure, replace the media fields with a short
   reason (`describe-failed`, `sdp-invalid`, …). Exit 0 when the scan
   completed (including zero peers); non-zero for missing interface,
   Avahi connect failure, or other hard errors.

7. **Module layout.** Prefer a small `browse` module (or extend `avahi`
   + a thin `rtsp` client helper) called from `main`, keeping formatting
   unit-testable without D-Bus. Do not block the daemon tick on this
   path.

## Risks / Trade-offs

- **[Risk] mDNS settle misses late advertisements** → Mitigation:
  configurable `--timeout`; document that a second run may differ.
- **[Risk] Peer RTSP is slow or hangs** → Mitigation: per-DESCRIBE
  connect/read timeout (a few seconds); continue other sessions.
- **[Risk] Host resolves to a name that lacks A record on the AES67
  NIC** → Mitigation: prefer Avahi resolve address bytes (IPv4) over
  re-resolving hostnames via system DNS.
- **[Risk] Shared browse helpers tempt wiring sinks in the same PR** →
  Mitigation: explicit non-goal; daemon tick keeps the empty map.

## Migration Plan

Ship the updated `obake-ravenna` binary via the existing package/OTA
path. No config migration. Operators gain `obake-ravenna browse`;
`run` / `status` unchanged.

## Open Questions

None deferred; default settle of 3s and orphan-session listing can be
tuned during implementation without changing the requirements.
