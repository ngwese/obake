# Spec Delta

## ADDED Requirements

### Requirement: SSH login realtime limits checked

After a successful SSH login as `obake` in QEMU, the verification SHALL
confirm that the login shell has realtime priority limit 95 and unlimited
locked memory, without the harness setting those limits itself.

#### Scenario: ulimit -r is 95 after SSH login

- **WHEN** the harness has logged in over SSH as `obake` on the booted host
- **THEN** `ulimit -r` in that login shell equals 95

#### Scenario: memlock is unlimited after SSH login

- **WHEN** the harness has logged in over SSH as `obake` on the booted host
- **THEN** `ulimit -l` in that login shell reports unlimited

## MODIFIED Requirements

### Requirement: User bus reachable over SSH

After a successful SSH login as `obake` in QEMU, the verification SHALL confirm
that `systemctl --user` can talk to the lingering user manager without the
harness exporting user-bus environment variables. The check SHALL rely on PAM
session environment, not on a `profile.d` workaround in the image.

#### Scenario: systemctl --user works after SSH login

- **WHEN** the harness has logged in over SSH as `obake` on the booted host
- **THEN** `systemctl --user status` succeeds in that session without setting
  `XDG_RUNTIME_DIR` or `DBUS_SESSION_BUS_ADDRESS` in the harness command
