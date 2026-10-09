# Spec Delta

## ADDED Requirements

### Requirement: System CA trust store checked after boot

After a successful boot, the verification SHALL confirm that the installed
image exposes a system CA trust store usable by host TLS clients, so a
missing or empty store fails headless verification before release.

#### Scenario: CA trust store present in guest checks

- **WHEN** the installed host has booted in QEMU and guest functional checks
  run
- **THEN** the harness confirms the system CA trust store is present and
  non-empty
