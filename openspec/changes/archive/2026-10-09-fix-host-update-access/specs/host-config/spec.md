# Spec Delta

## ADDED Requirements

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

## MODIFIED Requirements

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
