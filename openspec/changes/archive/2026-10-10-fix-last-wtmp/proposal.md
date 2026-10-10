# Proposal

## Why

`last` fails on the host with `can't open '/var/log/wtmp': No such file or
directory`, so operators cannot inspect login history while debugging. `/var`
is a tmpfs, systemd's shipped tmpfiles never create the wtmp family, and
Buildroot builds OpenSSH with utmp/wtmp disabled, so even an empty file would
not receive SSH login records.

## What Changes

- Create `/var/log/wtmp` (and related btmp/lastlog) at boot so `last` can open
  its database.
- Rebuild OpenSSH with utmp/wtmp/lastlog recording enabled so SSH sessions are
  written into that database.
- Persist login accounting under the writable `/var` contract so `last` history
  survives reboot and slot switch (same backing as the journal).
- Extend headless SSH verification to assert `last` works after login.
- Document the persisted login-accounting paths in host-config docs.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `host-config`: require login accounting files under `/var/log` so `last`
  works, SSH sessions are recorded, and those records persist with the
  defined `/var` subset.
- `image-verification`: require the SSH guest check to confirm `last` succeeds
  and shows the `obake` login after an SSH session.

## Impact

- `image/os/buildroot/board/obake/rootfs-overlay/etc/tmpfiles.d/`: create
  wtmp/btmp/lastlog on the `/var` tmpfs each boot.
- `image/os/buildroot/board/obake/rootfs-overlay/etc/obake/persisted-var`:
  widen `log/journal` to `log` so wtmp (and journal) persist on `/persist`.
- `image/os/buildroot/external.mk`: drop Buildroot's OpenSSH
  `--disable-{utmp,utmpx,wtmp,wtmpx,lastlog}` flags (rebuilds sshd).
- `image/verification/qemu/ssh-test.sh` (and guest checks if needed).
- `image/docs/host-config.md`: document login accounting in the persisted
  `/var` subset.
- OTA: hosts pick this up on the next slot image; existing
  `/persist/var/log/journal` remains valid when the bind root becomes
  `/persist/var/log`.
