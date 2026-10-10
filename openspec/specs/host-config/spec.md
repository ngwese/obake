# host-config Specification

## Purpose
Keeps the operating system root immutable and update-safe while giving users a
well-defined, persistent place to customize configuration, so that user changes
survive updates and updates never touch user data.

## Requirements

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

The system SHALL define a directory contract that names the locations that are
writable for host configuration and user data. A user connecting over SSH as
`obake` SHALL be able to write under `/home`, and SHALL NOT be able to write
`/etc` or the read-only operating system root. System services MAY still write
the `/etc` overlay and other contract mounts as root.

#### Scenario: User customizes within the contract

- **WHEN** a user connected over SSH as `obake` modifies a file under
  `/home/obake`
- **THEN** the change persists on the user partition across boots and updates

#### Scenario: User cannot write /etc over SSH

- **WHEN** a user connected over SSH as `obake` attempts to create or modify a
  file under `/etc`
- **THEN** the modification fails

#### Scenario: User cannot modify outside the contract

- **WHEN** a user connected over SSH as `obake` attempts to modify the
  read-only root outside the contract
- **THEN** the modification fails

### Requirement: Host user privileged access

The `obake` login user SHALL be able to perform system maintenance over SSH
without interactive authentication: managing systemd units and power state,
reading host diagnostics — the kernel log via `dmesg` and the full system
journal via `journalctl` — and applying or rolling back signed OS updates via
`obake-update`. Privileged update actions SHALL use a narrowly scoped
passwordless sudo grant for `/usr/bin/obake-update` only, not a general root
shell and not unrestricted `mount`/`umount`.

#### Scenario: Unit and power management over SSH

- **WHEN** the obake user runs `systemctl` unit or power operations over SSH
- **THEN** the action succeeds without a password prompt

#### Scenario: Kernel log readable

- **WHEN** the obake user runs `dmesg`
- **THEN** the kernel ring buffer is readable

#### Scenario: Full system journal readable

- **WHEN** the obake user reads the system journal with `journalctl`
- **THEN** entries from all services and the whole boot are readable

#### Scenario: Update apply without a password prompt

- **WHEN** the obake user runs `obake-update apply` over SSH with a valid
  bundle available on auto-mounted media or the configured HTTPS source
- **THEN** the update proceeds without a password prompt

#### Scenario: No general root via sudo

- **WHEN** the obake user runs `sudo -i` or `sudo /bin/sh`
- **THEN** sudo denies the command

### Requirement: Locked root account

The image SHALL lock the root account so it has no usable password and local
`su` to root without credentials fails, while still allowing the system to
run root-owned services.

#### Scenario: su to root fails without credentials

- **WHEN** the `obake` user runs `su -` with an empty or incorrect password
- **THEN** authentication fails and the session remains unprivileged

#### Scenario: Root has no usable password

- **WHEN** an image is built
- **THEN** the root account password field is locked (not empty and not a
  valid hash an operator can type)

### Requirement: Removable media auto-mount for obake

When a removable storage partition appears, the system SHALL mount it
automatically for the `obake` user under `/media/<name>`, where `<name>` is
the filesystem label when present, or a stable fallback derived from the
volume identity when the volume is unlabeled. The mount SHALL be readable by
`obake` without additional privilege.

#### Scenario: Labeled removable volume mounts under /media

- **WHEN** a removable partition with filesystem label `UPDATES` is attached
- **THEN** it is mounted at `/media/UPDATES` and the `obake` user can read
  files there without sudo

#### Scenario: Unlabeled removable volume still mounts

- **WHEN** a removable partition without a filesystem label is attached
- **THEN** it is mounted under `/media/` with a stable non-empty directory
  name derived from the volume identity

#### Scenario: Removal unmounts cleanly

- **WHEN** the removable media is detached
- **THEN** the corresponding `/media/<name>` mount is removed
