# system-install Specification

## Purpose
Provisions an obake host from an installer medium without a graphical
interface, establishing the A/B boot layout and the persistent configuration
partitions that later updates and user customization depend on.

## Requirements

### Requirement: Headless installer

The installer SHALL operate over a serial console or a text console with an
attached keyboard and monitor, SHALL support selecting which console the
installer medium uses at build time, and SHALL NOT require a graphical
interface at any point.

#### Scenario: Install on a headless host

- **WHEN** the installer runs on a host with no display server
- **THEN** all prompts, selections, and progress are available over the
  selected serial or text console

#### Scenario: Install over serial

- **WHEN** the installer medium is built with the serial console selected
- **THEN** all prompts, selections, and progress are available over the serial
  console

#### Scenario: Install over a local text console

- **WHEN** the installer medium is built with the local console selected and
  runs on a host whose firmware provides a framebuffer
- **THEN** all prompts, selections, and progress are available on the attached
  monitor and the attached keyboard drives the installer

### Requirement: Installer console selection

The installer build SHALL select the medium's interactive console between
serial and local via the build-time `CONSOLE` setting, SHALL default to
serial so headless CI and QEMU verification are unchanged, and SHALL reject
an unrecognized selection.

#### Scenario: Default selection is serial

- **WHEN** an installer medium is built without specifying a console
- **THEN** it presents the installer over the serial console

#### Scenario: Local console selected

- **WHEN** an installer medium is built with the local console selection
- **THEN** it presents the installer on a local virtual terminal and accepts
  input from an attached USB keyboard

#### Scenario: Unrecognized selection rejected

- **WHEN** an installer medium is built with an unrecognized console selection
- **THEN** the build fails with an error naming the supported selections

### Requirement: Post-install shutdown wait

After the installer finishes (success or failure), it SHALL wait before
powering off the host: when built for interactive target-disk selection it
SHALL prompt on the selected console for operator confirmation; when built
non-interactive it SHALL wait a build-time duration that defaults to 10
seconds and MAY be set to zero for automated tooling.

#### Scenario: Interactive confirm before poweroff

- **WHEN** the installer medium was built for interactive disk selection and
  the install script finishes
- **THEN** the installer prompts on the selected console for confirmation and
  powers off only after the operator confirms

#### Scenario: Non-interactive default delay

- **WHEN** the installer medium was built with a fixed target disk and no
  custom poweroff delay, and the install script finishes
- **THEN** the installer waits 10 seconds on the selected console and then
  powers off

#### Scenario: Non-interactive zero delay for automation

- **WHEN** the installer medium was built with a poweroff delay of zero and
  the install script finishes
- **THEN** the installer powers off without waiting

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

The installer SHALL install a unified kernel image for each slot and create
exactly one active firmware boot entry for each slot, labeled `A` and `B`,
pointing at that slot's UKI on the ESP it just wrote. Before creating those
entries it SHALL remove every existing firmware boot entry whose label is `A`
or `B`, including entries for a previous ESP and entries that are not active.
Entries with any other label SHALL be left in place. The resulting boot order
SHALL try the install's active slot before the other slot.

#### Scenario: First boot uses active slot

- **WHEN** installation completes and the host powers on
- **THEN** the firmware loads the active slot's unified kernel image and the
  corresponding root slot

#### Scenario: Reinstall does not keep stale slot entries

- **WHEN** the installer runs on a machine that already has one or more EFI
  entries labeled `A` or `B`, including entries that point at a different disk
  or are inactive
- **THEN** those entries are removed, exactly one active `A` and one active `B`
  remain, and both point at the UKIs on the ESP written by this install

#### Scenario: Other firmware entries survive install

- **WHEN** the firmware has boot entries whose labels are not `A` or `B`
- **THEN** the installer does not delete those entries

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
