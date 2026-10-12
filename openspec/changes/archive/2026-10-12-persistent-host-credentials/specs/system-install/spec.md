# Spec Delta

## ADDED Requirements

### Requirement: Installer seeds the obake authorized key

When the installer medium carries an SSH authorized key seed, the installer
SHALL provision that key as the `obake` user's authorized key in that user's
home on the user partition, with the home's `.ssh` directory and the key file
owned by `obake` and not readable or writable by other users. The installer
SHALL provision the key only when the user has no authorized key already and
SHALL NOT overwrite an existing key, so a reinstall that preserves user data
leaves a user-managed key unchanged. When the medium carries no seed, the
installer SHALL provision no authorized key.

#### Scenario: Seed provisioned on a fresh install

- **WHEN** the installer runs against a fresh disk using a medium that carries
  an authorized key seed
- **THEN** the `obake` user's home on the user partition contains that key in
  its `.ssh` directory, owned by `obake`

#### Scenario: Existing user key preserved on reinstall

- **WHEN** the installer runs against a disk whose user partition already has
  an `obake` authorized key and the install preserves user data
- **THEN** the existing key is left unchanged

#### Scenario: No seed provisions no key

- **WHEN** the installer runs using a medium that carries no authorized key
  seed
- **THEN** no authorized key is provisioned for `obake`
