# Spec Delta

## ADDED Requirements

### Requirement: ESP readable by obake

The EFI System Partition mounted at `/boot/efi` SHALL be readable and listable
by the `obake` login user without additional privilege. The `/boot` path prefix
SHALL be traversable by that user so ordinary diagnostics (for example `df`)
can resolve the mount without Permission denied.

#### Scenario: obake can list the ESP

- **WHEN** the `obake` user runs `ls /boot/efi` after boot
- **THEN** the command succeeds and lists ESP contents without sudo

#### Scenario: df can resolve the ESP mount

- **WHEN** the `obake` user runs `df` after boot
- **THEN** the command does not fail with Permission denied for `/boot/efi`

#### Scenario: Non-root cannot write the ESP

- **WHEN** the `obake` user attempts to create a file under `/boot/efi`
  without sudo
- **THEN** the write fails
