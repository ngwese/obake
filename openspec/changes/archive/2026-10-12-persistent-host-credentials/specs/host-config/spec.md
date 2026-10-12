# Spec Delta

## ADDED Requirements

### Requirement: obake manages its own login credentials

Authorized keys for the `obake` login user SHALL be read from
`~/.ssh/authorized_keys` on the user partition. The `obake` user SHALL be able
to add, replace, or remove those keys without elevated privileges, and SHALL be
able to change its own account password without elevated privileges when the
account already has a password. Setting a password on an account that has none
(an image built without `OBKA_PASSWORD`, whose password field is locked) MAY
require elevation. Credentials the user sets SHALL persist across boots, slot
switches, and operating system updates, and SHALL NOT be overwritten by an
update. The image SHALL NOT provision a permanently honored authorized key
outside the user's home, so replacing the user's key retires any seed.

#### Scenario: obake replaces its authorized key without elevated privileges

- **WHEN** the `obake` user writes its own `~/.ssh/authorized_keys` over an
  interactive or remote session
- **THEN** the write succeeds without sudo and subsequent SSH logins use the
  new key

#### Scenario: Replacing the key retires the seed

- **WHEN** the `obake` user replaces the contents of `~/.ssh/authorized_keys`
  after an install seeded a different key
- **THEN** the seeded key no longer grants SSH access

#### Scenario: obake changes an existing password without elevated privileges

- **WHEN** the `obake` account already has a password and the `obake` user
  changes it
- **THEN** the change succeeds without sudo and the new password works for
  console and SSH password login

#### Scenario: First password on a key-only host requires elevation

- **WHEN** the `obake` account has no password (an image built without
  `OBKA_PASSWORD`) and the `obake` user runs `passwd` without elevation
- **THEN** the change is refused, and setting the password requires the host's
  existing sudo grant or a build-time `OBKA_PASSWORD` seed

#### Scenario: User-set credentials survive an update

- **WHEN** the `obake` user has set its own key or password and the host
  applies an operating system update or switches slots
- **THEN** the user-set key and password remain in effect
