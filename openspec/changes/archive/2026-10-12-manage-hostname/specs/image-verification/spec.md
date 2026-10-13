# Spec Delta

## ADDED Requirements

### Requirement: Hostname seeding and rename checked after boot

Verification SHALL assert that an image built with a hostname seed boots with
that name, that the `obake` user can change the host name over SSH without
elevated privileges, and that the change persists across a slot switch.

#### Scenario: Seeded host name on first boot

- **WHEN** an image built with a hostname seed is installed and booted
- **THEN** the host reports the seeded name

#### Scenario: obake renames without elevated privileges

- **WHEN** the `obake` user sets a new valid host name over SSH without sudo
- **THEN** the host reports the new name

#### Scenario: Rename survives a slot switch

- **WHEN** the `obake` user has renamed the host and the host then activates
  the other slot
- **THEN** the host still reports the new name
