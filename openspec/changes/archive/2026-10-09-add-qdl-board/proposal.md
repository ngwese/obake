# Proposal

## Why

obake needs a first-class target for the MeLE Quieter DL (Intel N100, dual
Intel I226-V), a compact fanless host suited to headless MIDI and future
audio-over-IP workloads. Today only generic x86_64 (QEMU/CI), generic aarch64,
and rk35xx exist; none carry Quieter DL drivers or MIDI 2.0 kernel support.

## What Changes

- Add a new Buildroot board `qdl` that shares `obake_base` and the pinned
  7.2.9 realtime kernel, differing only in board fragment, overlay, and a
  minimal custom kernel config.
- Enable USB MIDI 2.0 (UMP) and the hardware modules needed for Quieter DL
  (NVMe, IGC/I226, HDA; HDMI/DP audio via HDA, no i915 under PREEMPT_RT).
- Include AOIP-oriented kernel plumbing (PTP, PHY timestamping, XDP sockets)
  without AES67 modules, userspace, or advanced RT network tuning.
- Omit wireless (Wi-Fi / Bluetooth) drivers.
- Keep shared `NO_HZ_IDLE` from the project realtime fragment (no `NO_HZ_FULL`).
- Complete the pinned offline download cache (`vendor/dl/`) with the sources
  the shared base and x86_64 EFI path require but the aarch64-derived snapshot
  lacked.
- Disable Go VCS stamping for the Singularity build via a shared package patch,
  which the x86_64 host build surfaced.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `os-image-build`: add the `qdl` board as a supported target with a
  hardware-minimal kernel that includes MIDI 2.0 and AOIP network plumbing
  while remaining on the shared pinned base.

## Impact

- `image/os/buildroot/`: new `configs/obake-qdl.config`,
  `board/obake/qdl/` (linux.config + overlay), docs and `build.sh` board list.
- `image/os/buildroot/package/singularity/0002-go-disable-vcs-stamping.patch`:
  shared package patch adding `-buildvcs=false` so the Singularity Go build
  does not attempt VCS stamping under Buildroot's `GIT_DIR=.`.
- `image/os/buildroot/vendor/dl/`: complete the git-lfs-tracked offline cache
  with the sources newly required by the shared base and x86_64 EFI path
  (`gptfdisk`, `dosfstools`, `parted`, `lvm2`, `go` linux-amd64, `gnu-efi`,
  `python-pyelftools`).
- No pin bumps (kernel stays 7.2.9). No shape or installer contract changes.
- OTA/RAUC and partition layout unchanged; `qdl` uses EFI + systemd-boot like
  `generic-x86_64`.
- Cross-host portability of shapes is unchanged; board identity is recorded in
  `/etc/obake/board`.
