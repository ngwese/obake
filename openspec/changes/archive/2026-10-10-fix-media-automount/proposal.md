# Proposal

## Why

Removable media auto-mount shipped in `fix-host-update-access` does not work
on a real host: `/media` is a directory on the read-only root, so creating
per-volume mountpoints fails with `Read-only file system`. Even after that is
fixed, mounting from a udev `RUN+` under systemd-udevd's `PrivateMounts=yes`
can leave mounts invisible to the host namespace that `obake` and Singularity
bind mounts use. Operators cannot rely on `/media/<label>` for updates or
container access.

## What Changes

- Make `/media` a writable volatile mount (tmpfs or equivalent) so mountpoint
  directories can be created and removed on the read-only root image.
- Perform volume mounts in the host mount namespace (via systemd oneshot /
  `SYSTEMD_WANTS` / `systemd-mount`), not via `mount` inside udev `RUN+`, so
  mounts are visible to `obake` and to Singularity binds of `/media`.
- Keep udevd `PrivateMounts` isolation; do not weaken udevd unless a host-NS
  path proves insufficient.
- Tighten guest verification so `/media` is asserted writable (a mountpoint),
  not merely present as an empty directory.
- Document `/media` as volatile mount-root behavior for operators and shapes.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `host-config`: require a writable `/media` mount root and host-visible
  auto-mounts usable by `obake` and bindable into Singularity containers;
  clarify directory-contract treatment of `/media` as volatile.
- `image-verification`: guest checks must fail when `/media` is not a writable
  mount root suitable for automount.

## Impact

- `image/os/buildroot/board/obake/rootfs-overlay/etc/fstab` (or a `.mount`
  unit): tmpfs (or equivalent) on `/media`.
- `etc/udev/rules.d/99-obake-media-automount.rules` and
  `usr/libexec/obake/media-automount`: hand off mount/umount to a host-NS
  systemd path; improve failure visibility.
- Optional systemd template unit under the rootfs overlay.
- `etc/obake/directory-contract` and `image/docs/host-config.md` (plus related
  user-guide / os-update mentions if needed).
- `image/verification/guest/obake-verify.sh`: writable `/media` check.
- OTA: next slot image picks up the fix; no persist migration. Path stays
  `/media/<label>` (no Singularity bind path change).
