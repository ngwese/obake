# Proposal

## Why

Merging devices and software drivers place each session on a different
IPv4 multicast group with a shared RTP port, deriving the last two
octets from the host's unicast address. `obake-ravenna` today always
shares one group and steps the port. Operators who want Anubis-like
session destinations must pin every source by hand, and the main spec
forbids automatic address stepping. A named `merging` allocator makes
that observed layout a first-class option without inventing a generic
"address" scheme yet.

## What Changes

- Add a `[multicast]` allocation mode. Default remains today's port
  stepping (`port`). New mode `merging` assigns
  `239.{session_base + index}.{unicast_octet_3}.{unicast_octet_4}` with
  a shared even `base_port`.
- Derive the unicast octets from the PTP interface IPv4 already used
  for SDP and egress. When that address is missing, unpinned sources
  stay down.
- Keep per-source `address` / `port` pins as overrides under either
  mode.
- Update docs and examples so the two layouts are explicit. Existing
  configs that omit `allocation` keep current behavior (not
  **BREAKING**).

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `obake-ravenna`: transmit destinations may use either port stepping
  or Merging-style address stepping derived from the PTP unicast.

## Impact

- `services/obake-ravenna` config resolve path, qualify (destination
  uniqueness), tests, README / REFERENCE_GUIDE / examples.
- Persistent `config.toml` gains optional `allocation` and
  `session_base`. Cross-host copies of a `merging` config follow each
  host's unicast, which is intentional.
- Image seed config stays on default `port` unless operators opt in.
- No shape or OTA package changes.
