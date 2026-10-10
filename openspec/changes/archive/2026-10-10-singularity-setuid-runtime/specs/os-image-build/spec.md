# Spec Delta

## MODIFIED Requirements

### Requirement: Kernel namespaces for container runtime

Every board kernel SHALL enable Linux namespaces including user, PID, UTS,
IPC, network, and cgroup namespaces, so Singularity can create namespaces for
fakeroot builds, optional unprivileged flows, and container entry.

#### Scenario: User namespace supported

- **WHEN** a built image boots
- **THEN** creating a user namespace succeeds for an unprivileged process
  (for example the `obake` login user)

#### Scenario: Companion namespaces enabled

- **WHEN** a board kernel is built with the shared host fragment
- **THEN** PID, UTS, IPC, network, and cgroup namespaces are available in
  addition to user namespaces

### Requirement: Singularity runnable as host login user

The image SHALL ship a setuid SingularityCE helper and SHALL allow the `obake`
login user to run a local SIF using kernel SquashFS mounts (not a FUSE
squashfuse mount as the primary path), without requiring a root login.

#### Scenario: Local SIF runs as obake

- **WHEN** the `obake` user runs `singularity run` (or equivalent) against a
  local SIF on a booted image
- **THEN** the container starts successfully and the SIF root filesystem is
  mounted with the kernel SquashFS driver rather than squashfuse/FUSE

#### Scenario: Setuid helper present

- **WHEN** a built image is inspected
- **THEN** the Singularity setuid starter binary is present, owned by root, and
  mode `4755`, and `singularity.conf` is owned by root and not writable by
  non-root users, with setuid and kernel SquashFS mounts allowed

## ADDED Requirements

### Requirement: Fakeroot maps for on-device Singularity builds

The image SHALL provision subordinate UID and GID maps for the `obake` login
user so that user can run Singularity with `--fakeroot` on the device (for
example `singularity build --fakeroot` or `singularity exec --fakeroot`).

#### Scenario: subuid and subgid present for obake

- **WHEN** a built image boots
- **THEN** `/etc/subuid` and `/etc/subgid` each contain a mapping for `obake`
  with at least 65536 IDs

#### Scenario: Fakeroot exec as obake

- **WHEN** the `obake` user runs `singularity exec --fakeroot` against a local
  SIF with a command that prints the effective user id
- **THEN** the command reports uid `0` inside the container
