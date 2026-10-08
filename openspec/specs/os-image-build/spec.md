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
