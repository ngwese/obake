# Spec Delta

## MODIFIED Requirements

### Requirement: Directory-contract customization

The system SHALL define a directory contract that names the locations that are
writable for host configuration and user data. A user connecting over SSH as
`obake` SHALL be able to write under `/home`, and SHALL NOT be able to write
`/etc` or the read-only operating system root. System services MAY still write
the `/etc` overlay and other contract mounts as root.

#### Scenario: User customizes within the contract

- **WHEN** a user connected over SSH as `obake` modifies a file under
  `/home/obake`
- **THEN** the change persists on the user partition across boots and updates

#### Scenario: User cannot write /etc over SSH

- **WHEN** a user connected over SSH as `obake` attempts to create or modify a
  file under `/etc`
- **THEN** the modification fails

#### Scenario: User cannot modify outside the contract

- **WHEN** a user connected over SSH as `obake` attempts to modify the
  read-only root outside the contract
- **THEN** the modification fails
