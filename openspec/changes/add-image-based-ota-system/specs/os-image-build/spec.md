# Spec Delta

## Purpose

Builds bootable obake OS images from replaceable layers so that the base system
can be rebased for security updates without reworking kernel, tuning, or runtime
work, and emits versioned, signed artifacts the update system can install.

## ADDED Requirements

### Requirement: Layered reproducible build

The system SHALL build OS images from ordered, independently replaceable layers
consisting of at least a minimal base distribution, a kernel layer, a tuning
layer, and a runtime layer. Replacing the base layer with a newer release SHALL
not require modifying the higher layers.

#### Scenario: Base rebase without higher-layer changes

- **WHEN** the base layer is bumped to a newer distribution release
- **THEN** the image rebuilds using unchanged kernel, tuning, and runtime layer
  definitions

#### Scenario: Reproducible artifact

- **WHEN** the same layer definitions and pinned inputs are built twice
- **THEN** the resulting image artifacts are byte-identical

### Requirement: Custom realtime kernel

Each image SHALL include a custom kernel configured for realtime audio
performance and SHALL version that kernel together with the image it is built
into.

#### Scenario: Realtime tuning present in image

- **WHEN** an image is built
- **THEN** the included kernel carries the project's realtime configuration and
  the image reports a kernel version tied to the image version

### Requirement: Per-board tuning without forking the base

The build SHALL support hardware-specific tuning for distinct target boards
while sharing a single common base, so that adding or changing a board does not
fork the base system.

#### Scenario: Two boards share one base

- **WHEN** images are built for two different target boards
- **THEN** both images derive from the same base layer and differ only in the
  board-specific tuning layer

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

The image SHALL include a Singularity CE runtime built from source at a pinned
version, because distribution-provided packages are too old or absent.

#### Scenario: Pinned Singularity CE present

- **WHEN** an image is built
- **THEN** the runtime layer provides the pinned Singularity CE version built
  from source rather than a distribution package
