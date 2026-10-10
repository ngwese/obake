# Spec Delta

## ADDED Requirements

### Requirement: Optional network cabling checked after boot

After a successful boot, the verification SHALL confirm that wired networkd
configuration does not require managed interfaces for online, so a wait-online
failure from missing carrier would be caught before release.

#### Scenario: Guest sees optional wired online requirement

- **WHEN** guest functional checks run after boot
- **THEN** the suite fails unless the active wired `.network` configuration
  marks managed interfaces as not required for online

### Requirement: User bus reachable over SSH

After a successful SSH login as `obake` in QEMU, the verification SHALL confirm
that `systemctl --user` can talk to the lingering user manager without the
harness exporting user-bus environment variables.

#### Scenario: systemctl --user works after SSH login

- **WHEN** the harness has logged in over SSH as `obake` on the booted host
- **THEN** `systemctl --user status` succeeds in that session without setting
  `XDG_RUNTIME_DIR` or `DBUS_SESSION_BUS_ADDRESS` in the harness command
