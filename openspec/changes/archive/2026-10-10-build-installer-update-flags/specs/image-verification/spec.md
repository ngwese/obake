# Spec Delta

## ADDED Requirements

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
  any explicit installer overrides

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

### Requirement: Default development signing material

When signing certificate, key, or keyring paths are unset, the build entry
point SHALL use development signing material under the image repository's
`_build/signing` directory, creating that material if it is missing, so a
board installed from that host accepts update bundles later produced on the
same host without manually supplying keys. Explicit signing environment
variables SHALL override the defaults.

#### Scenario: Update build creates signing material when absent

- **WHEN** an operator builds with `--update` and no signing environment
  variables, and the default signing directory has no material
- **THEN** the build creates development signing material there and emits
  bundles signed with it

#### Scenario: Installer bakes the default keyring when unset

- **WHEN** an operator builds with `--installer` and no keyring environment
  variable
- **THEN** the built image trusts the default development CA from
  `_build/signing`

#### Scenario: Explicit signing overrides defaults

- **WHEN** an operator sets signing certificate, key, or keyring paths
  explicitly
- **THEN** the build uses those paths instead of the default development
  material
