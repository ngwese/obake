# Proposal

## Why

The host OS image is built from a moving Debian trixie mirror and a stock
distribution kernel, so rebuilding it for a new board can silently change a
system that already works, and the custom realtime kernel the design called for
was never built. The build also produces x86_64 images by running an amd64
container under emulation, which is being removed from the next macOS release
and is unavailable on the project's arm64 Linux hosts.

## What Changes

- Replace the Debian `mmdebstrap` layered base with a Buildroot-based build that
  pins a Buildroot release and every package version and source hash, so inputs
  are deterministic across rebuilds and across boards. **BREAKING** for the
  image build pipeline.
- Build the realtime kernel from a pinned Linux version with a project config
  fragment, completing the deferred custom realtime kernel work; the kernel is
  no longer a stock distribution package.
- Provide Singularity CE as a native, in-tree Buildroot package built from a
  pinned source release, so the runtime remains built from source.
- Keep the minimal base userland on systemd and fold the current tuning-layer
  packages into that base.
- Cross-compile target images from any Linux host without executing target
  binaries or emulation, and decouple building from verification so verification
  consumes a built artifact set and may run on a different host using the best
  available accelerator. On macOS arm64 the aarch64 image is the iteration
  target, verified under QEMU with hardware acceleration as a proxy for the
  arm64 board.
- Retain the per-slot unified kernel image and the RAUC A/B update model, signed
  bundles, health gate, and rollback unchanged.
- Retain the writable contract (read-only root, persistent `/etc` overlay,
  `/var` tmpfs, persist and user partitions) and the installer, re-hosted on the
  Buildroot base.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `os-image-build`: the build basis changes from replaceable Debian layers to a
  pinned Buildroot tree with deterministic inputs; the realtime kernel is built
  from source rather than taken from a distribution package; Singularity CE is
  provided by a native in-tree Buildroot package; per-slot unified kernel images
  are retained.
- `image-verification`: the build no longer depends on target-architecture
  emulation; build and verification are decoupled with the built artifact set as
  the interface; verification uses the best accelerator available and may run on
  a host other than the build host; the verification architecture is native-first
  with aarch64 as the macOS arm64 proxy target.

## Impact

- `image/os`: `build.sh`, `manifest.json`, `layers/*`, and `boards/*` are
  replaced by a Buildroot `BR2_EXTERNAL` tree, board defconfigs, and package
  recipes; `rauc/system.conf`, `partition-layout.json`, and `scripts/*` are
  largely retained.
- `image/verification`: the build environment becomes an arm64-native
  cross-build container without QEMU or OVMF; verification scripts consume an
  artifact directory; the QEMU harnesses are retained.
- `image/installer`: the installer image is built from the Buildroot base; the
  partition and install logic is retained.
- `image/host`: the udev and JACK configuration is retained and installed via a
  Buildroot rootfs overlay or package.
- Persistent configuration, OTA upgrades, and cross-host portability are
  preserved: the persist and user contract, the RAUC slot model, and the bundle
  format are unchanged, and the image is no longer tied to a Debian mirror.
