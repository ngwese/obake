# Proposal

## Why

Configured `[[sink]]` entries stay `device-absent` because the daemon
never fills the discovery map that qualify already expects. The one-shot
`browse` command can see Anubis/Merging sessions and DESCRIBE their SDP
(with path encoding); the control loop still passes an empty map every
tick.

## What Changes

- Wire peer discovery into the session daemon: browse RAVENNA devices
  and sessions on the PTP interface, DESCRIBE sessions needed for
  configured sinks that are not yet established (shared percent-encoded
  RTSP client), and feed `(device, stream) → StreamSdp` into qualify so
  matching sinks can come up.
- Once a sink is up, keep that SDP in cache — do not re-DESCRIBE or
  re-resolve the session on every control tick or discovery refresh.
- Share discovery/DESCRIBE orchestration between `browse` and the
  daemon so path encoding and SDP parse behavior stay consistent
  (including `a=framecount` ranges).
- Document the behavior in the service reference guide.
- No config schema, image packaging, or OTA seed changes beyond the
  updated binary.

## Capabilities

### New Capabilities

- (none)

### Modified Capabilities

- `obake-ravenna`: the daemon SHALL discover peer RAVENNA sessions and
  supply their SDP to sink qualify (closing the existing
  `device-absent` gap), without polling SDP for sinks that are already
  up.

## Impact

- `services/obake-ravenna/`: discovery cache + background (or equivalent)
  refresh that DESCRIBEs only unsettled sinks; tick uses the shared map;
  refactor so `browse` and the daemon call the same DESCRIBE/path
  helpers.
- Observable: a correctly named `[[sink]]` can leave `device-absent` and
  join when PTP and SDP checks pass (as already specified).
- No shape or persistent-config format changes. OTA delivers the binary.
