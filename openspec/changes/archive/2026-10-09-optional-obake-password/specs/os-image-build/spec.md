# Spec Delta

## ADDED Requirements

### Requirement: Optional build-time obake password

The build SHALL accept an optional cleartext password for the `obake` login
account via the `OBKA_PASSWORD` environment variable. When unset or empty, the
account SHALL have no password and SSH password authentication as `obake`
SHALL fail. When set, the built image SHALL accept that password for local
console login and SHALL allow SSH password authentication as `obake` with that
password. The password SHALL live only in the image rootfs, not on the persist
partition.

#### Scenario: Default has no password

- **WHEN** an image is built without `OBKA_PASSWORD` set
- **THEN** the `obake` account has no usable password and SSH password
  authentication as `obake` fails

#### Scenario: Password set enables console and SSH password login

- **WHEN** an image is built with a non-empty `OBKA_PASSWORD`
- **THEN** the `obake` account accepts that password for local console login
  and an SSH client can authenticate as `obake` using that password

#### Scenario: Password is not persist state

- **WHEN** an image built with `OBKA_PASSWORD` is installed
- **THEN** the password hash is not written to the persist partition and is
  replaced when a different slot image is activated
