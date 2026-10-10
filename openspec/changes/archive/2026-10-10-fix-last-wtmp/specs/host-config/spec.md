# Spec Delta

## ADDED Requirements

### Requirement: Login accounting for last

The system SHALL provide login accounting files under `/var/log` so that the
`last` command can open `/var/log/wtmp` without error. After an SSH login as
`obake`, `last` SHALL report that session. Those accounting files SHALL be part
of the persisted `/var` subset so records survive reboot and slot switch.

#### Scenario: last opens wtmp

- **WHEN** the `obake` user runs `last` after the host has finished boot
- **THEN** the command does not fail with a missing `/var/log/wtmp` error

#### Scenario: SSH session appears in last

- **WHEN** the `obake` user has logged in over SSH and then runs `last`
- **THEN** the output includes an entry for the `obake` user

#### Scenario: Login records survive reboot

- **WHEN** an SSH login as `obake` has been recorded and the host reboots
- **THEN** `last` still shows that prior `obake` session
