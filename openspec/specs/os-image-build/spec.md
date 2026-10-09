# os-image-build Specification

## Purpose
Builds bootable obake OS images from a single pinned Buildroot tree so that
every input is deterministic across rebuilds and boards, and emits versioned,
signed artifacts the update system can install.

## Requirements

### Requirement: Deterministic pinned build

The build SHALL derive every input from a pinned Buildroot release and pinned
package versions and source hashes, so that rebuilding with unchanged pins
selects the same base userland, kernel, and runtime regardless of when or where
the build runs, and SHALL emit a reproducible root filesystem archive as its
canonical artifact.

#### Scenario: Same pins select the same inputs

- **WHEN** the image is rebuilt with unchanged pins at a later time or on
  another host
- **THEN** the same package versions, kernel version, and runtime version are
  selected

#### Scenario: Stable inputs across boards

- **WHEN** an image is rebuilt for a new board from the same pins
- **THEN** the shared base userland, kernel, and runtime are unchanged and only
  the board-specific configuration differs

#### Scenario: Reproducible artifact

- **WHEN** the same pinned inputs are built twice
- **THEN** the resulting root filesystem archive is reproducible

### Requirement: Minimal systemd base userland

The image SHALL provide a minimal, systemd-based userland that includes at
least a shell, common administration tools, and the Singularity runtime needed
by the host contract, and SHALL NOT require a distribution package manager at
runtime.

#### Scenario: Required tools present

- **WHEN** an image is built
- **THEN** the userland provides `bash`, `grep`, `git`, `vi`, `sudo`, `ip`,
  `systemctl`, `journalctl`, `dmesg`, and `singularity`

#### Scenario: No package manager dependency

- **WHEN** the running host is used
- **THEN** no runtime package manager is required for the host contract to
  function

### Requirement: Custom realtime kernel

Each image SHALL include a realtime kernel built from a pinned kernel source
with a project realtime configuration fragment, and SHALL version that kernel
together with the image it is built into.

#### Scenario: Realtime tuning present in image

- **WHEN** an image is built
- **THEN** the included kernel is built from the pinned source with the
  project's realtime configuration and the image reports a kernel version tied
  to the image version

### Requirement: Per-board tuning without forking the base

The build SHALL support hardware-specific tuning for distinct target boards
while sharing a single common base, so that adding or changing a board does not
fork the base system.

#### Scenario: Two boards share one base

- **WHEN** images are built for two different target boards
- **THEN** both images derive from the same base Buildroot configuration and
  differ only in the board's defconfig and overlay

### Requirement: Per-slot unified kernel image

Each image SHALL produce a unified kernel image (UKI) bundling the kernel,
initial ramdisk, and kernel command line for the slot the image targets.

#### Scenario: UKI corresponds to its slot

- **WHEN** an image is built for a given slot
- **THEN** the emitted UKI embeds the kernel and command line for that slot and
  is named so the boot entry can associate it with that slot

### Requirement: Versioned signed artifacts

The build SHALL emit versioned image artifacts together with a signature that
the update system can verify before installing them.

#### Scenario: Signed artifact verifiable

- **WHEN** a build completes
- **THEN** it produces a versioned image artifact and a signature over it that
  the update system can validate

### Requirement: Singularity CE runtime built from source

The build SHALL provide a pinned Singularity CE runtime built from source, so
that the runtime does not depend on a distribution package.

#### Scenario: Pinned Singularity CE present

- **WHEN** an image is built
- **THEN** the runtime provides the pinned Singularity CE version built from
  source rather than a distribution package

### Requirement: Local display console for hardware boards

A board with no serial console SHALL include a kernel build that drives a text
console from the firmware-provided EFI framebuffer, so an attached monitor and
USB keyboard can be used without a GPU driver that the realtime configuration
excludes.

#### Scenario: Firmware framebuffer text console

- **WHEN** the qdl image is built
- **THEN** the kernel binds a framebuffer console to the EFI framebuffer and
  provides a virtual terminal for local output

#### Scenario: Console does not require an RT-incompatible GPU driver

- **WHEN** the qdl kernel is built with the realtime fragment
- **THEN** the local console works without i915, which the realtime
  configuration excludes

### Requirement: Board build profiles

Each board SHALL provide a `default` and a `virtual` build profile. A profile
supplies the board's installer and console build defaults (`MAKE_INSTALLER`,
`CONSOLE`, `INSTALLER_TARGET_DISK`); `default` targets the physical board and
`virtual` targets an emulation or verification target, and the generic
boards' two profiles are identical. When no profile is named the build SHALL
use `default`, an explicit setting SHALL override the profile value, and an
unknown profile SHALL fail the build.

#### Scenario: Default profile selected

- **WHEN** a board is built without naming a profile
- **THEN** it applies the `default` profile's installer and console defaults

#### Scenario: Virtual profile selected

- **WHEN** a board is built naming the `virtual` profile
- **THEN** it applies that profile's emulation-oriented installer and console
  defaults

#### Scenario: Explicit setting overrides the profile

- **WHEN** an installer or console setting is also provided explicitly with a
  profile
- **THEN** the explicit value takes precedence over the profile value

#### Scenario: Generic profiles are equivalent

- **WHEN** a generic board is built with either profile
- **THEN** the resulting build configuration is the same

#### Scenario: Unknown profile rejected

- **WHEN** a build names a profile that the board does not provide
- **THEN** the build fails with an error naming the supported profiles

### Requirement: Profile console for installer and installed OS

The build-time `CONSOLE` setting (`serial` or `local`) SHALL select the
interactive console for both the installer medium and the installed OS
unified kernel images, including the kernel command line and getty, so a
board profile's console choice is consistent from install through first boot.

#### Scenario: Local console on installer and installed OS

- **WHEN** a board is built with `CONSOLE=local`
- **THEN** the installer medium and each installed OS UKI present on the local
  text console

#### Scenario: Serial console on installer and installed OS

- **WHEN** a board is built with `CONSOLE=serial`
- **THEN** the installer medium and each installed OS UKI present on the
  board's serial console

#### Scenario: qdl default uses local end-to-end

- **WHEN** the qdl board is built with its default profile
- **THEN** both the installer medium and the installed OS UKIs use the local
  text console

### Requirement: Board-implied target architecture

Building a board SHALL derive the target architecture from the board's
configuration, so the operator does not supply an architecture separately.

#### Scenario: Architecture follows the board

- **WHEN** a board is built without an architecture argument
- **THEN** the emitted image targets the architecture declared by that board

#### Scenario: Distinct architectures from distinct boards

- **WHEN** a board that targets x86_64 and a board that targets aarch64 are
  built the same way
- **THEN** each emitted image targets its own board's architecture

### Requirement: Optional build-time obake password

The build SHALL accept an optional cleartext password for the `obake` login
account via the `OBKA_PASSWORD` environment variable. When unset or empty, the
account SHALL have no password and SSH password authentication as `obake`
SHALL fail. When set, the built image SHALL accept that password for local
console login and SHALL allow SSH password authentication as `obake` with that
password. The password SHALL live only in the image rootfs, not on the persist
partition.

#### Scenario: Default has no password

- **WHEN** an image is built without `OBKA_PASSWORD` set
- **THEN** the `obake` account has no usable password and SSH password
  authentication as `obake` fails

#### Scenario: Password set enables console and SSH password login

- **WHEN** an image is built with a non-empty `OBKA_PASSWORD`
- **THEN** the `obake` account accepts that password for local console login
  and an SSH client can authenticate as `obake` using that password

#### Scenario: Password is not persist state

- **WHEN** an image built with `OBKA_PASSWORD` is installed
- **THEN** the password hash is not written to the persist partition and is
  replaced when a different slot image is activated
