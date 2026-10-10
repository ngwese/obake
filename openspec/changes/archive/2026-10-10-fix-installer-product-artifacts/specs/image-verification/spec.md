# Spec Delta

## MODIFIED Requirements

### Requirement: Build reports artifact locations

The build SHALL print the location of the artifacts it produces when it
completes, and SHALL be able to report the artifact directory without
performing a build. While copying artifacts out of the build cache and while
removing derived images from that cache, the build SHALL print a progress line
before each of those steps so the wait is visible.

#### Scenario: Locations printed

- **WHEN** a build completes
- **THEN** it prints the artifact directory and the produced artifact paths

#### Scenario: Directory reported without building

- **WHEN** the artifact directory is requested without building
- **THEN** the build prints the path and does not run a build

#### Scenario: Staging and cache cleanup announce themselves

- **WHEN** a containerized build copies artifacts out of the cache volume or
  deletes derived images from that volume
- **THEN** a progress line is printed before that step starts

### Requirement: Build product selection

The build entry point SHALL require exactly one product flag, `--installer` or
`--update`, when performing a build. Omitting both or passing both SHALL fail
with a usage error. Path discovery without building (`--print-dir`) SHALL NOT
require a product flag. Board profiles (`default` / `virtual`) SHALL continue to
supply installer console and target-disk defaults independently of the product
flag.

#### Scenario: Installer product selected

- **WHEN** an operator builds a board with `--installer`
- **THEN** the build emits installer media according to the board profile and
  any explicit installer overrides, and does not emit `.raucb` bundles unless
  bundle generation is explicitly requested

#### Scenario: Update product selected

- **WHEN** an operator builds a board with `--update`
- **THEN** the build emits signed RAUC update bundles and does not emit
  installer media or a full bootable disk image unless explicitly overridden

#### Scenario: Missing product flag rejected

- **WHEN** an operator invokes the build entry point without `--installer` or
  `--update`
- **THEN** the build fails with a usage error naming the required flags

#### Scenario: Both product flags rejected

- **WHEN** an operator passes both `--installer` and `--update`
- **THEN** the build fails with a usage error

#### Scenario: Print-dir needs no product flag

- **WHEN** an operator asks the build entry point only for the artifact
  directory path
- **THEN** the path is printed without requiring `--installer` or `--update`
