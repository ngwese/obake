# Spec Delta

## Purpose

Provisions an obake host from an installer medium without a graphical
interface, establishing the A/B boot layout and the persistent configuration
partitions that later updates and user customization depend on.

## ADDED Requirements

### Requirement: Headless installer

The installer SHALL operate over a serial console or a text console with an
attached keyboard and monitor, and SHALL NOT require a graphical interface at
any point.

#### Scenario: Install on a headless host

- **WHEN** the installer runs on a host with no display server
- **THEN** all prompts, selections, and progress are available over the serial
  or text console

### Requirement: Target disk selection and confirmation

The installer SHALL enumerate candidate destination disks, require the operator
to select one, and SHALL require explicit confirmation before performing any
destructive operation.

#### Scenario: Operator selects and confirms a disk

- **WHEN** the operator selects a destination disk
- **THEN** the installer presents the target and its pending destruction and
  proceeds only after an affirmative confirmation

#### Scenario: Operator declines confirmation

- **WHEN** the operator does not confirm the selected disk
- **THEN** the installer performs no writes and exits without modifying any disk

### Requirement: Stable partition layout

The installer SHALL create a partition layout comprising an EFI system
partition, two root slots, a persist partition, and a user partition, using
stable partition labels and GUIDs that are identical across installs and
architectures.

#### Scenario: Layout uses stable identifiers

- **WHEN** the installer partitions a disk
- **THEN** the created partitions carry the project's stable labels and GUIDs
  regardless of the target device or architecture

### Requirement: A/B boot installation

The installer SHALL install a unified kernel image for each slot and create a
firmware boot entry for each, and the installed host SHALL boot the active slot
by default.

#### Scenario: First boot uses active slot

- **WHEN** installation completes and the host powers on
- **THEN** the firmware loads the active slot's unified kernel image and the
  corresponding root slot

### Requirement: Machine-specific seeding

The installer SHALL seed the persist partition with machine-specific state
including a machine identifier, SSH host keys, hostname, and network
configuration, and SHALL create the user partition.

#### Scenario: Persist seeded at install

- **WHEN** installation completes
- **THEN** the persist partition contains a unique machine identifier, SSH host
  keys, and initial hostname and network configuration, and the user partition
  exists and is mountable

### Requirement: Reinstall preserving user data

The installer SHALL be able to reinstall the operating system onto an existing
disk while preserving the existing user partition, so that reset and recovery
do not destroy user data.

#### Scenario: Reinstall keeps user partition

- **WHEN** the installer runs against a disk that already has the stable layout
- **THEN** it recreates the system partitions and leaves the user partition and
  its contents intact
