# Proposal

## Why

Debugging on a live host often needs an interactive root shell (`sudo -s`) and
arbitrary privileged commands. Today `obake` only has passwordless sudo for
`/usr/bin/obake-update`, so operators cannot escalate for diagnostics without
unlocking root or rebuilding the image.

## What Changes

- **BREAKING**: Expand the `obake` sudoers grant from `/usr/bin/obake-update`
  only to passwordless sudo for all commands, including interactive shells
  (`sudo -s`, `sudo -i`, `sudo /bin/sh`).
- Keep the root account locked: empty-password `su` to root still fails; sudo
  is the intended escalation path for the login user.
- Update headless guest checks and docs so they expect shell sudo to succeed
  instead of denying it.
- Leave the unprivileged directory contract unchanged: without sudo, `obake`
  still writes under `/home` and cannot write `/etc` or the read-only root.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `host-config`: broaden host-user privileged access from a narrow
  `obake-update` sudo grant to passwordless sudo for all commands, including
  root shells.
- `image-verification`: flip guest sudo checks so shell sudo is allowed; keep
  asserting that empty-password `su` to root fails.
- `os-update`: drop the "limited sudo grant" wording; apply/rollback as
  `obake` still works via the broader passwordless sudo.

## Impact

- `image/os/buildroot/board/obake/rootfs-overlay/etc/sudoers.d/obake-update`
  (or a renamed drop-in): grant `NOPASSWD: ALL` for `obake`.
- `image/verification/guest/obake-verify.sh`: assert shell sudo succeeds.
- Docs (`image/docs/os-update.md`, host-config / user-guide if they describe
  the narrow grant).
- OTA: hosts pick up the broader sudoers with the next slot image; no persist
  migration. Security posture on SSH-reachable hosts widens for anyone who
  can authenticate as `obake`.
