# Proposal

## Why

obake currently has no defined way to build, install, or update the host OS.
Hosts are provisioned by hand, the base system drifts per machine, and there is
no safe mechanism to ship security fixes or new kernels without risking an
unbootable device or clobbering user data. This blocks the core promises of
persistent configuration and reliable OTA upgrades.

## What Changes

- Introduce a layered, reproducible OS image build on a minimal Debian base,
  with a custom realtime-tuned kernel and a Singularity CE runtime built from
  source. Rebasing on a newer base becomes a mechanical rebuild.
- Introduce a headless, serial/text-console installer that partitions a target
  disk with stable partition labels/GUIDs and installs an A/B slot pair.
- Introduce user-initiated OTA updates via RAUC over a custom signed HTTPS
  endpoint, with USB-drive sideload preferred when media is present (airgapped
  and AoIP-only hosts). Bundles are X.509 signed and carry a per-slot UKI so
  kernels roll back with userspace.
- Introduce a read-only root with a hybrid writable contract: an `/etc` overlay
  and persisted `/var` subset on a shared persist partition, and a separate
  user partition untouched by updates and restores.
- Introduce a health-gate service that snapshots and restores user
  configuration, validates an update over a bounded number of boots, and
  automatically reverts to the previous slot on failure.
- Introduce a reproducible, containerized build environment that runs on the
  project's development hosts (macOS arm64 and Windows 11 WSL x86_64) and a
  headless QEMU verification harness that installs, boots, and exercises the
  x86_64 image before release.

## Capabilities

### New Capabilities

- `os-image-build`: layered minimal-Debian image build producing versioned,
  signed artifacts (rootfs, custom realtime kernel, per-slot UKI) with board
  tuning and a Singularity-CE-from-source runtime.
- `system-install`: headless installer that selects a target disk, applies a
  stable partition layout, installs the A/B slots and their UKIs, and seeds
  the persist and user partitions.
- `os-update`: user-initiated RAUC A/B updates delivered by signed HTTPS or USB
  sideload, with per-slot UKIs, native slot activation and rollback, and a
  health gate.
- `host-config`: read-only root with an `/etc` overlay, tmpfs `/var` plus a
  persisted subset, a user partition untouched by updates/restores, a
  directory-contract customization surface, and narrowly scoped privileged
  access for the host user (systemd/power control and diagnostics).
- `image-verification`: a containerized build environment that produces
  identical x86_64 images on macOS arm64 and Windows 11 WSL x86_64, plus a
  headless QEMU harness that installs, boots, and functionally checks the image.

### Modified Capabilities

- None.

## Impact

- New build tooling, partition/install tooling, a RAUC-based update client, and
  a QEMU verification harness added to the project; no existing specs are
  affected.
- x86_64 is the primary target and the focus of QEMU verification; arm64 remains
  a supported target built from the same layers.
- `host/` udev rules and JACK configuration become part of the tuned image
  layer rather than manual per-host installation.
- Persistent configuration and cross-host portability now depend on the persist
  and user partition contract; shapes remain independent and out of scope.
- Firmware updates (fwupd UEFI capsules) and shape packaging are explicitly out
  of scope for this change.
