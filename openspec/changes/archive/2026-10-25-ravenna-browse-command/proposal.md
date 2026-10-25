# Proposal

## Why

Configuring a `[[sink]]` or verifying peer announcements requires knowing
which RAVENNA devices and sessions are visible on the AES67 NIC, and what
each session advertises in SDP. Today the operator has no first-party
one-shot tool for that; the daemon's browse path is stubbed and serves
only the control loop.

## What Changes

- Add an `obake-ravenna browse` CLI subcommand: a one-shot human
  diagnostic that lists RAVENNA devices and sessions on the PTP
  interface and prints a short SDP summary per session (sample rate,
  channel count, format, frame size, and multicast group/port).
- The command talks to Avahi and peer RTSP directly. It does not require
  the session daemon, and it does not feed discovery into sink qualify.
- Session DESCRIBE uses the TXT `path=` when present, otherwise
  `/by-name/<session instance name>`.
- Document the command in the service README (and reference guide as
  needed). No persistent configuration, image packaging, or OTA behavior
  changes beyond shipping the updated binary.

## Capabilities

### New Capabilities

- (none)

### Modified Capabilities

- `obake-ravenna`: add a one-shot `browse` diagnostic that enumerates
  RAVENNA devices and sessions and summarizes each session's SDP.

## Impact

- `services/obake-ravenna/`: new CLI path in `main.rs`, Avahi browse /
  resolve client work, a small RTSP DESCRIBE client, and report
  formatting. Reuses existing `parse_sdp` / `StreamSdp`.
- Does not change the daemon control loop, PipeWire modules, PTP gating,
  config schema, or sink qualify. Daemon browse remains stubbed for
  receive until a later change.
- No shape, installer, or persistent-config impact. OTA delivers the
  binary as usual; no new unit or seed.
- Headless verification: unit tests for path selection, grouping, and
  report formatting; live board check against a real peer when available.
