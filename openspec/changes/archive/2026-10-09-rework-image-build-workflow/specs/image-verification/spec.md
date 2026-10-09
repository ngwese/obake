# Spec Delta

## ADDED Requirements

### Requirement: Build tooling separate from verification

The build entry point and its containerized build environment SHALL be provided
outside the verification tree, so that build and verification are decoupled and
verification consumes the emitted artifact set rather than build internals.

#### Scenario: Build entry point outside verification

- **WHEN** an operator looks for the build tooling
- **THEN** the build entry point and build environment live in a dedicated
  build area separate from the verification harnesses

#### Scenario: Verification consumes artifacts only

- **WHEN** verification runs
- **THEN** it consumes the artifact directory and does not depend on the build
  tooling's internal layout

### Requirement: Build outputs within the image repository

Every host-visible build output directory SHALL live inside the image
repository, SHALL have a name beginning with an underscore, and SHALL be
ignored by version control.

#### Scenario: Outputs inside the repository

- **WHEN** the build runs from a clone of the image repository alone
- **THEN** every output directory is created inside that repository tree

#### Scenario: Underscore directories ignored

- **WHEN** output directories have been created by a build
- **THEN** version control ignores them

### Requirement: Host-owned build outputs

Files written from the build container to the host SHALL be owned by the user
who invoked the build, so the user can read, move, and remove them without
elevated privileges.

#### Scenario: Container outputs owned by the invoking user

- **WHEN** a containerized build completes
- **THEN** the staged files on the host are owned by the invoking user

#### Scenario: Cleanup needs no elevated privileges

- **WHEN** the invoking user removes a completed build's output directory
- **THEN** the removal succeeds without elevated privileges

### Requirement: Unique per-board artifact directory

Each build SHALL stage its artifacts in a directory whose path identifies the
board, the profile, and the build revision, so that builds for different boards
or profiles do not overwrite one another.

#### Scenario: Two boards of one architecture do not collide

- **WHEN** two boards that share a target architecture are built
- **THEN** their artifact directories are distinct

#### Scenario: Virtual and default do not collide

- **WHEN** the same board is built with the `virtual` and `default` profiles
- **THEN** their artifact directories are distinct

#### Scenario: Revision recorded in the path

- **WHEN** the image repository has no uncommitted changes
- **THEN** the artifact directory path includes the image version and the short
  revision; when it has uncommitted changes the short revision is marked dirty

### Requirement: Build reports artifact locations

The build SHALL print the location of the artifacts it produces when it
completes, and SHALL be able to report the artifact directory without
performing a build.

#### Scenario: Locations printed

- **WHEN** a build completes
- **THEN** it prints the artifact directory and the produced artifact paths

#### Scenario: Directory reported without building

- **WHEN** the artifact directory is requested without building
- **THEN** the build prints the path and does not run a build
