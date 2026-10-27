# Proposal

## Why

Updating an obake host today means manually probing the active slot over SSH,
picking the matching `obake-<semver>-<slot>.raucb`, copying it, running
`obake-update apply`, deleting the staged file, and rebooting. That ritual is
easy to get wrong (wrong slot, stale version) and fails when operators need it
most—when higher-level daemons are broken. A single SSH-backed CLI command
should own selection, push, apply, cleanup, and reboot.

## What Changes

- Add `obake device update --target user@host PATH [--force]` to the `obake`
  CLI. Transport is SSH only (no GraphQL / session API), so updates work when
  `obake-session` is down.
- When `PATH` is a directory, probe the device for running version and active
  slot, then select the latest `obake-<semver>-<inactive-slot>.raucb` whose
  semver is greater than or equal to the device version.
- When `PATH` is a file, parse slot and semver from the filename and refuse a
  wrong-slot or older bundle unless `--force` is set.
- Push the chosen bundle over SSH/SCP, then run
  `obake-update apply --cleanup --reboot <staged-path>` on the device.
- Extend on-device `obake-update apply` with optional `--cleanup` (remove the
  caller-supplied bundle path after a successful install) and `--reboot`
  (run `systemctl reboot` after a successful install, after cleanup when both
  are set).
- Document the new CLI verb and the apply flags. No `device describe` in this
  change.

## Capabilities

### New Capabilities

- (none)

### Modified Capabilities

- `os-update`: `obake-update apply` gains `--cleanup` and `--reboot`; cleanup
  applies only to an explicit caller-supplied bundle path; reboot runs only
  after a successful install (and after cleanup when both flags are set).
- `obake-session`: the `obake` CLI gains a `device` noun with `update`, using
  required `--target user@host`, directory/file selection rules, `--force`,
  and SSH push + `obake-update apply --cleanup --reboot`.

## Impact

- `image/os/buildroot/board/obake/rootfs-overlay/usr/bin/obake-update` and
  update docs (`image/docs/os-update.md`, user-guide).
- `services/obake` CLI (`main.rs`, new device/SSH helper module, README,
  tests).
- OTA operator workflow: preferred path becomes `obake device update` instead
  of hand-rolled scp + apply + reboot. Existing USB/HTTPS/`apply <path>`
  flows remain.
- No change to RAUC slot layout, signing, health gate, or session GraphQL.
