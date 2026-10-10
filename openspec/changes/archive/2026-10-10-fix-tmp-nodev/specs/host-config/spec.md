# Spec Delta

## ADDED Requirements

### Requirement: Volatile /tmp allows device nodes

The system SHALL mount `/tmp` as a tmpfs that is writable by the `obake`
login user, discarded across boots, mounted `nosuid`, and not mounted
`nodev` or `noexec`. The directory contract SHALL name `/tmp` as that
volatile mount.

#### Scenario: /tmp is a tmpfs without nodev

- **WHEN** a built image boots
- **THEN** `/tmp` is a tmpfs whose mount options include `nosuid` and do not
  include `nodev` or `noexec`, and `obake` can create a file there

#### Scenario: /tmp does not survive reboot

- **WHEN** `obake` creates a file under `/tmp` and the host reboots
- **THEN** that file is absent

#### Scenario: Contract lists /tmp as volatile

- **WHEN** an operator reads the directory contract
- **THEN** `/tmp` is named as a volatile tmpfs that is not persisted
