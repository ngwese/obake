# Spec Delta

## ADDED Requirements

### Requirement: Credential seeding and self-service checked after boot

Verification SHALL assert that an image built with a supplied authorized key
seeds the `obake` user's key at install and that the user can log in with it,
that the `obake` user can replace that key and change its own password without
elevated privileges, and that both changes persist across a slot switch.

#### Scenario: Seeded key grants first login

- **WHEN** an image built with a supplied authorized key is installed and
  booted
- **THEN** an SSH client presenting the matching private key logs in as
  `obake` without a password prompt

#### Scenario: Replacing the key retires the seed

- **WHEN** the `obake` user replaces its authorized key and the host reboots
- **THEN** the new key logs in and the seeded key no longer does

#### Scenario: obake changes its own password without elevated privileges

- **WHEN** the `obake` user changes its own password over SSH without sudo
- **THEN** the new password works for a later SSH password login

#### Scenario: Credential changes survive a slot switch

- **WHEN** the `obake` user has replaced its key and changed its password and
  the host then activates the other slot
- **THEN** the new key and password remain in effect
