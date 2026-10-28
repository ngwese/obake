# Proposal

## Why

Operators cannot see which instances belong to the active session from
`obake session describe`, and cannot read or follow shape stdout/stderr
without SSH and journalctl. When a container exits immediately (missing
work file, bad args), that failure is invisible from the CLI. Singularity
already writes per-instance `.out`/`.err` logs; they need a first-class
control-plane path.

## What Changes

- Active `session` GraphQL results include the session rig's instance
  names (and enough fields for `obake session describe` to list them).
- The service streams those instances' Singularity log files over HTTP
  for the **active session only**, with optional follow.
- Zero instance names multiplexes every active instance with a
  `[name]` line prefix; one or more names filters to those instances.
- `obake session log [--follow] [INSTANCE...]` consumes that stream.
- Log retention stays whatever Singularity already does (files are not
  deleted on stop; v1 does not serve stopped sessions).

## Capabilities

### New Capabilities

- (none)

### Modified Capabilities

- `obake-session`: Active session describe exposes instances; HTTP log
  stream and CLI `session log` / describe output for those instances.

## Impact

- `services/obake-session`: GraphQL `Session` shape; new HTTP log
  endpoint on the same listeners as file transfer; resolve Singularity
  log paths for running instances.
- `services/obake`: `session describe` formatting; new `session log`
  command and client HTTP streaming.
- No shape image, host OS, OTA, or persistent config layout changes.
- No new log store under the share tree in v1.
