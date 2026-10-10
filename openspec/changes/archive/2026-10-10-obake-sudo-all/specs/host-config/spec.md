# Spec Delta

## MODIFIED Requirements

### Requirement: Host user privileged access

The `obake` login user SHALL be able to perform system maintenance over SSH
without interactive authentication: managing systemd units and power state,
reading host diagnostics — the kernel log via `dmesg` and the full system
journal via `journalctl` — and applying or rolling back signed OS updates via
`obake-update`. Privileged actions that need root SHALL use a passwordless
sudo grant for all commands, including interactive root shells (`sudo -s`,
`sudo -i`, `sudo /bin/sh`). The grant SHALL NOT require unlocking the root
account password.

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
- **THEN** sudo grants a root shell without a password prompt

#### Scenario: Arbitrary command via sudo

- **WHEN** the obake user runs `sudo -n <command>` for an arbitrary command
- **THEN** sudo runs the command as root without a password prompt
