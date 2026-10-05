# Spec Delta

## Purpose

Keeps the operating system root immutable and update-safe while giving users a
well-defined, persistent place to customize configuration, so that user changes
survive updates and updates never touch user data.

## ADDED Requirements

### Requirement: Read-only operating system root

The operating system root filesystem SHALL be mounted read-only during normal
operation, so that users cannot introduce changes to the root that would be
lost or would conflict with the next update.

#### Scenario: Root cannot be modified in place

- **WHEN** a process attempts to write to a path on the root filesystem that is
  not part of the writable contract
- **THEN** the write fails because the root is mounted read-only

### Requirement: Persistent configuration overlay

Changes made to system configuration under `/etc` SHALL persist across boots
and across slot switches by being stored in a writable overlay backed by the
persist partition.

#### Scenario: Configuration change survives reboot

- **WHEN** a user changes an allowed file under `/etc` and the host reboots
- **THEN** the change is still present

#### Scenario: Configuration survives slot switch

- **WHEN** the host switches from one slot to the other
- **THEN** the configuration stored in the overlay is unchanged

### Requirement: Volatile and persisted runtime state

The system SHALL provide a writable `/var` whose transient content is discarded
across boots and whose defined persisted subset survives across boots on the
persist partition.

#### Scenario: Transient state discarded

- **WHEN** the host reboots
- **THEN** transient `/var` content that is not part of the persisted subset is
  discarded

#### Scenario: Persisted subset retained

- **WHEN** the host reboots
- **THEN** the defined persisted subset under `/var` is retained

### Requirement: Persist partition

The system SHALL keep machine-specific state on a dedicated persist partition
that is shared across both slots and is not replaced by operating system
updates.

#### Scenario: Machine state independent of slot

- **WHEN** an operating system update activates the other slot
- **THEN** the machine-specific state on the persist partition is retained

### Requirement: Separate user partition untouched by updates

The system SHALL keep user data on a separate, writable user partition that is
mounted for user access and that SHALL NOT be modified by operating system
updates or by system reinstalls that preserve user data.

#### Scenario: Update leaves user data intact

- **WHEN** an operating system update is applied
- **THEN** no content on the user partition is modified

#### Scenario: Reinstall leaves user data intact

- **WHEN** the system is reinstalled while preserving user data
- **THEN** no content on the user partition is modified

### Requirement: Directory-contract customization

The system SHALL define a directory contract that names the locations a user may
customize, and a user connecting over SSH SHALL be able to modify configuration
only within those locations.

#### Scenario: User customizes within the contract

- **WHEN** a user connected over SSH modifies a file within a directory named by
  the contract
- **THEN** the change persists as defined by the writable contract

#### Scenario: User cannot modify outside the contract

- **WHEN** a user connected over SSH attempts to modify the read-only root
  outside the contract
- **THEN** the modification fails
