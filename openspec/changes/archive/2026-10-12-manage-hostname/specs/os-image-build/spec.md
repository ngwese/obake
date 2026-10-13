# Spec Delta

## ADDED Requirements

### Requirement: Build-time hostname seed

The build SHALL accept an optional host name for the installed host via the
`OBKA_HOSTNAME` environment variable. When set to a valid host name, the build
SHALL carry it as a seed that the installer medium provisions and that
`MAKE_DISK` images bake into their persist `/etc` overlay, and SHALL behave the
same on macOS and Linux build hosts. When unset or empty, the seed SHALL be the
default host name `obake`. When set to a value that is not a valid host name,
the build SHALL fail with an error naming `OBKA_HOSTNAME`.

#### Scenario: Seed carried by the installer medium

- **WHEN** an installer medium is built with `OBKA_HOSTNAME` set to a valid
  host name
- **THEN** the medium carries that name as the seed the installer provisions

#### Scenario: Seed baked into a disk image

- **WHEN** a disk image is built with `OBKA_HOSTNAME` set to a valid host name
- **THEN** the host boots with that name

#### Scenario: Invalid host name fails the build

- **WHEN** an image is built with `OBKA_HOSTNAME` set to a value that is not a
  valid host name
- **THEN** the build fails with an error naming `OBKA_HOSTNAME`

#### Scenario: Unset host name uses the default

- **WHEN** an image is built without `OBKA_HOSTNAME` set
- **THEN** the seed is the default host name `obake`
