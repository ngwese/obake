# Proposal

## Why

ANEMAN and the Merging MAD/VAD panels only put a host in the patching
matrix when it advertises a ZMAN-class HTTP service and serves session
sources, sinks, and channel names. `obake-ravenna` publishes RAVENNA
RTSP records, which Anubis already uses, and does not expose that HTTP
service. The page's control channel is Bayeux/CometD, and the service channels
after the handshake are unpublished, so the work has to grow from a
mock page plus a real handshake and captured traffic rather than a
guessed schema.

## What Changes

- Advertise `_merging_zman._sub._http._tcp` on the PTP interface, serve
  a mock `/advanced/index.html`, and accept a Bayeux/CometD handshake at
  `/cometd/handshake` on port 9090. The mDNS subtype stays
  `_merging_zman`.
- Record the Bayeux service channels from a live session and treat that
  capture as the schema. Do not invent channel names.
- Serve the configured sources and sinks, including a name for every
  channel, through the captured read API.
- Accept the captured "connect" request that attaches a discovered peer
  source to a configured sink.
- Leave the existing RTSP node and session records in place.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `obake-ravenna`: the daemon also appears as a ZMAN HTTP device, lists
  configured streams and channel names, and can attach a peer source to
  a configured sink.

## Impact

- `services/obake-ravenna` (HTTP listener, Avahi publish, qualify/sink
  target, tests, reference docs). Port 9090 needs no capability or
  sysctl change.
- No new third-party crate is required for the mock page. A later slice
  may add one only if the captured client needs behavior the standard
  library cannot provide.
- Persistent config may gain optional channel names; absent names use a
  generated label. Cross-host behavior moves with `config.toml`.
