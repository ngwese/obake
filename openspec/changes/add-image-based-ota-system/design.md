# Design

## Context

obake has no host OS lifecycle: hosts are hand-provisioned, the base drifts per
machine, and there is no safe way to ship a kernel or security fix. Shapes are
containerized and already portable; this design concerns only the host OS that
runs them. See proposal.md for motivation.

Constraints that shape the approach:

- Targets are x86_64 (primary) and arm64 (secondary). arm64 boards (e.g.
  FriendlyElec RK35xx) boot via U-Boot; U-Boot's EFI payload is assumed so both
  architectures present UEFI to a common boot model.
- Development/build hosts are macOS arm64 and Windows 11 WSL on x86_64, so the
  build and verification must be host-agnostic.
- The base must run Singularity CE, which the project already builds from source
  because distribution packages are too old or absent.
- The minimal base is Debian trixie, whose systemd (257) provides `ukify`, the
  tool used to generate unified kernel images. bookworm's systemd 252 does not.
- Hosts may be airgapped (AoIP-only) and updates are user-initiated.
- Kernel and userspace must roll back together.

## Goals / Non-Goals

**Goals:**

- One reproducible, layered image build that rebases cheaply onto a newer base.
- One A/B update model covering both architectures.
- A read-only root with a defined, persistent customization surface.
- Automatic revert of a failed update without losing user data.
- A containerized build and a headless QEMU harness that prove an x86_64 image
  installs, boots, and functions before release.

**Non-Goals:**

- Updating device/platform firmware (BIOS/EC/USB interfaces). fwupd is a
  separate, later change.
- Packaging, versioning, or updating shapes; shapes are independent.
- Encryption, verified boot (Secure Boot), and runtime integrity (dm-verity).
- Fleet management, staged rollout, or telemetry.
- Verifying arm64 under QEMU; x86_64 is the QEMU-verified architecture and arm64
  is validated on hardware.

## Decisions

### Update engine: RAUC over systemd-sysupdate

RAUC is chosen because its bootloader backends cover both targets from one
update model: `efi` (EFI `BootOrder`/`BootNext`) on x86_64 and U-Boot
(`bootchooser`) on arm64 if U-Boot's EFI payload is not usable.
systemd-sysupdate is cleaner on x86_64 UEFI but is effectively
systemd-boot-centric and has no first-class U-Boot rollback. RAUC also provides
signed bundles, slot state, and a D-Bus API.

- Alternative: systemd-sysupdate + systemd-boot (rejected: weak on U-Boot).
- Alternative: custom fwupd plugin for OS images (rejected: reimplements RAUC).

### Slots: A/B only, no recovery partition

Two root slots. Recovery is "reinstall the OS, keep the user partition" via the
installer rather than a dedicated recovery slot. This saves space and keeps a
single recovery path.

- Alternative: A/B + recovery slot (rejected: space, third slot to maintain).

### Boot manager and kernel placement: per-slot UKIs with RAUC activation

The kernel cannot live in the ext4 root slot when using a firmware/EFI boot
path, so each slot ships a unified kernel image (kernel + initrd + cmdline) on
the ESP, named per slot. The UKI is version-locked to its root slot, so kernel
and userspace roll back together. Slot selection uses RAUC's native bootloader
backends rather than systemd-boot: the `efi` backend manipulates EFI
`BootOrder`/`BootNext` on x86_64, and the `uboot` backend manipulates
`BOOT_ORDER` on arm64. Firmware boots the selected slot's UKI directly.

- Alternative: systemd-boot BLS with filename boot counting (rejected: RAUC's
  `efi` backend does not use it, so slot selection would not be driven by RAUC).
- Alternative: shared kernel on the ESP (rejected: kernel not A/B).
- Alternative: dedicated XBOOTLDR partition (deferred: more room only if UKIs
  grow).

### Slot activation and rollback via RAUC

RAUC's bootloader backend activates the updated slot after installation
(`BootNext`/`BootOrder` on `efi`, `BOOT_ORDER` on `uboot`), so the next boot
uses the inactive slot that was just written. The health gate marks the slot
good once the update proves healthy; if it does not within the configured number
of boots, it marks the new slot bad and activates the previous slot, and the
firmware boots the previous UKI. Slot state is stored on the persist partition
so it survives slot switches.

- Alternative: systemd-boot filename boot counting (rejected: not driven by
  RAUC, and requires a separate activation path).

### Health gate

A systemd service runs early on boot when an update is pending. It restores the
prior user configuration (shapes list, JACK profile, and the like) from a
pre-update snapshot on the persist partition, validates the running update over
a bounded number of boots, and on success marks the slot good through RAUC. If
the update is not marked good within the bound, the service marks the new slot
bad and activates the previous slot so the firmware boots it.

### Update transport: USB preferred, signed HTTPS otherwise

The update client accepts RAUC bundles from removable USB media and from a
configured signed HTTPS endpoint, using identical bundle format and signature
verification. USB is preferred when present, which serves airgapped hosts.
Updates run only on explicit user request.

### Trust model: signed bundles only

Bundles are X.509-signed and verified against a project keyring before any slot
write. No Secure Boot and no dm-verity; runtime integrity rests on the
read-only root mount. Accepted consciously to reduce scope.

### Image build: layered minimal Debian with packages

Build with `mmdebstrap` plus a local apt repository, expressing kernel, tuning,
runtime, and per-board layers as Debian packages/metapackages over a pinned
Debian trixie base. Rebasing is bumping the base pin and rebuilding; higher
layers are untouched.

- Alternative: mkosi (rejected: its UKI, ESP, signing, and disk-image pipeline
  is strongest with systemd-sysupdate, which was not chosen; mmdebstrap is
  Debian-native and bootloader-neutral, and RAUC consumes a rootfs image and UKI
  directly).
- Alternative: Buildroot (rejected: the Singularity CE runtime is not packaged
  and would be heavy to support).

### UKI generation: ukify on the trixie base

Generate each per-slot UKI with `ukify` during the build, using the systemd-stub
from the base. Trixie's systemd 257 provides `ukify`; this is why the base is
trixie rather than bookworm.

- Alternative: `dracut --uefi` (rejected: less direct control over the embedded
  cmdline and sections).
- Alternative: manual `objcopy` of `systemd-stub` (rejected: unnecessary on
  trixie).

### Build environment: containerized Linux

The build runs inside a Linux container so identical commands work on macOS
arm64 and Windows 11 WSL x86_64. x86_64 is the primary target; on macOS arm64
the container provides an amd64 environment, and on WSL it runs natively. The
container image and all inputs are pinned so the two hosts produce equivalent
artifacts.

- Alternative: native per-host tooling (rejected: diverges and undermines
  reproducibility).

### Verification: QEMU with UEFI firmware

A headless harness boots built images and drives the installer under QEMU with
OVMF UEFI firmware, using the serial or text console, and asserts boot plus core
behavior: read-only root, the writable contract, and an update apply and revert.
x86_64 is the default verified architecture; arm64 is validated on hardware.

- Alternative: hardware-in-the-loop for every run (rejected: slow and scarce);
  QEMU is the default gate, with hardware as a periodic smoke test.

### Partition layout with stable identifiers

Fixed layout with stable labels and GPT type GUIDs, identical on both
architectures, so the installer and reinstall path can locate partitions by
role.

| Role    | Label          | Contents                          | Mount      |
|---------|----------------|-----------------------------------|------------|
| ESP     | `OBKA_ESP`     | per-slot UKIs                    | `/boot/efi`|
| root A  | `OBKA_ROOT_A`  | read-only rootfs slot A           | `/`        |
| root B  | `OBKA_ROOT_B`  | read-only rootfs slot B           | `/`        |
| persist | `OBKA_PERSIST` | /etc overlay, persisted /var, seed| `/persist` |
| user    | `OBKA_USER`    | user data, home                   | `/home`    |

ext4 throughout, no encryption. ESP sized for the two per-slot UKIs
(start 1 GiB); root slots equal and fixed; persist modest and fixed; user takes
the remainder.

### Writable contract

- Root mounted read-only.
- `/etc` is an overlay whose upper and work directories live on the persist
  partition, so configuration persists across boots and slot switches.
- `/var` is tmpfs with a defined persisted subset bind-mounted from persist.
- The user partition is separate and is never modified by updates or by
  reinstalls that preserve user data.
- A directory contract enumerates the writable locations; SSH users can modify
  only within it.

### Build/repo layout

New top-level material, kept out of `shapes/`:

- `os/` (working name): image build definitions, layer packages, partition
  metadata, signing.
- `installer/` (working name): installer image and partitioning logic.
- `verification/` (working name): containerized build environment and the QEMU
  verification harness.
- `host/` udev and JACK configuration move into the tuning layer.

Exact paths are settled during implementation.

## Risks / Trade-offs

- [U-Boot's environment storage varies across boards] -> Use RAUC's `uboot`
  backend against the board's environment; validate on the arm64 board.
- [Both per-slot UKIs share the ESP] -> The ESP carries only UKIs, not a shared
  boot-manager binary; each update writes the inactive slot's UKI only.
- [EFI variable support may be incomplete or non-atomic] -> Use RAUC's `efi`
  backend, keep the previous slot bootable in `BootOrder`, and validate in QEMU;
  do not rely on EFI variable writes being atomic.
- [Shared persist across OS versions] -> Define state migrations; health gate
  restores a pre-update config snapshot on failure.
- [Health gate false-good or false-bad] -> Keep criteria minimal and observable
  (services up, audio path present) and make the number of validation boots
  configurable.
- [No dm-verity / Secure Boot] -> Read-only root limits runtime tampering;
  revisit if the threat model changes.
- [Power loss during slot write] -> Writes target the inactive slot; sync before
  activation; RAUC's bootloader slot state provides the final safety net.
- [Base rebase pulls incompatible kernel or Singularity CE changes] -> Pin all
  inputs;
  rebuild and test in qemu per architecture before release.
- [Cross-arch build emulation on macOS arm64] -> Pin the container image, treat
  WSL x86_64 as the reference build, and compare artifacts across hosts.
- [QEMU/OVMF differs from real firmware] -> Keep a periodic hardware smoke test;
  do not rely on QEMU alone for firmware-specific behavior.

## Migration Plan

Greenfield: no existing hosts are migrated.

1. Build the layered image pipeline, stand up the containerized build
   environment, and produce a bootable x86_64 image.
2. Build the installer and validate install and boot with the QEMU harness
   (x86_64); validate arm64 on hardware.
3. Integrate RAUC, signing, USB/HTTPS delivery, and native slot activation.
4. Add the health gate and validate a forced-failure revert.
5. Only then treat the build as the supported provisioning path; `host/` manual
   setup is retired into the tuning layer.

Rollback of the change itself is simply not adopting the new build; no running
system depends on it until step 5.

## Open Questions

- Exact health-gate criteria and try count (product-specific; configurable, does
  not change the approach).
- Whether the persist `/etc` overlay needs explicit whiteout handling for
  removed configuration (implementation detail).
- Final ESP and root-slot sizes (tune from measured UKI and rootfs sizes).
- QEMU acceleration strategy on macOS arm64 (hardware acceleration vs software
  emulation); does not change the approach.
