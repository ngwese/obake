# Spec Delta

## ADDED Requirements

### Requirement: Login accounting checked over SSH

After a successful SSH login as `obake` in QEMU, the verification SHALL confirm
that `last` runs without a missing-wtmp error and that its output includes the
`obake` user for the current session.

#### Scenario: last works after SSH login

- **WHEN** the harness has logged in over SSH as `obake` on the booted host
- **THEN** `last` succeeds and its output includes `obake`
