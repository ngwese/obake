# Proposal

## Why

As `obake`, `df` fails with `Permission denied` on `/boot/efi` because the
ESP mount uses `umask=0077` and the `/boot` mountpoint hierarchy can be
owner-only. That blocks ordinary diagnostics for the login user even though
ESP content is not secret on this host and `obake` already has passwordless
sudo for any privileged action.

## What Changes

- Ensure `/boot` and the `/boot/efi` mountpoint directory on the image are
  world-traversable (`0755`, root-owned).
- Mount the ESP (`OBKA_ESP` at `/boot/efi`) with a VFAT umask that allows
  unprivileged read and directory listing (e.g. `umask=0022`), while keeping
  ownership as root so non-root still cannot write UKIs or loaders.
- Add a guest functional check that `obake` can access `/boot/efi` without
  sudo (so `df` and similar tools do not fail on that mount).

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `host-config`: require that the ESP mount at `/boot/efi` is readable and
  listable by the `obake` login user without additional privilege, and that
  the `/boot` path prefix is traversable for that access.
- `image-verification`: guest checks must fail when `obake` cannot access
  `/boot/efi` without sudo.

## Impact

- `image/os/buildroot/board/obake/rootfs-overlay/etc/fstab`: ESP mount
  options (`umask=0077` → open read access).
- `image/os/buildroot/board/obake/post-build.sh` and/or
  `post-fakeroot.sh`: explicit `0755` on `/boot` and `/boot/efi`.
- `image/verification/guest/obake-verify.sh`: access check as `obake`.
- Docs that describe ESP mount options if they mention `umask=0077`.
- OTA: next slot image picks up the change; no persist migration. RAUC and
  `obake-efi-entries` continue to write the ESP as root.
