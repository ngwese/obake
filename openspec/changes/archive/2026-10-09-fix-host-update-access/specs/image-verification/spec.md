# Spec Delta

## ADDED Requirements

### Requirement: Root lock and update sudo checked after boot

After a successful boot, the verification SHALL confirm that the root account
is locked and that the `obake` user may invoke `obake-update` via passwordless
sudo without being granted a general root shell.

#### Scenario: Root password field locked

- **WHEN** the installed host has booted in QEMU and guest functional checks
  run
- **THEN** the harness confirms the root account password field is locked

#### Scenario: obake-update sudo allowed, shell sudo denied

- **WHEN** the installed host has booted in QEMU and guest functional checks
  run
- **THEN** the harness confirms `sudo -n obake-update status` succeeds as
  `obake` and `sudo -n /bin/sh -c true` is denied
