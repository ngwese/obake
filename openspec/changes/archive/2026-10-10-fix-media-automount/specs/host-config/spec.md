## MODIFIED Requirements

### Requirement: Directory-contract customization

The system SHALL define a directory contract that names the locations that are
writable for host configuration and user data, including volatile `/media` as
the removable-media mount root. A user connecting over SSH as `obake` SHALL be
able to write under `/home`, and SHALL NOT be able to write `/etc` or the
read-only operating system root. System services MAY still write the `/etc`
overlay and other contract mounts as root.

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

#### Scenario: Media mount root is listed as volatile

- **WHEN** an operator reads the directory contract
- **THEN** `/media` is named as the volatile removable-media mount root

### Requirement: Removable media auto-mount for obake

When a removable storage partition appears, the system SHALL mount it
automatically under `/media/<name>` in the host mount namespace, where `<name>`
is the filesystem label when present, or a stable fallback derived from the
volume identity when unlabeled. `/media` SHALL be a writable mount root so
mountpoint directories can be created on the read-only OS image. The mount
SHALL be readable by `obake` without additional privilege and SHALL be
visible to processes that bind-mount `/media` (including Singularity).

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

#### Scenario: Media root accepts mountpoint directories

- **WHEN** the host has finished boot
- **THEN** `/media` is a writable mount root (not a directory on the
  read-only root alone) so a privileged process can create and remove a
  directory under `/media`

#### Scenario: Mount is visible for container bind

- **WHEN** a removable volume is auto-mounted under `/media/<name>`
- **THEN** that path is present in the host mount namespace so a Singularity
  bind of `/media` can expose the volume inside the container
