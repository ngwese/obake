# Spec Delta

## MODIFIED Requirements

### Requirement: Installer product emits installer media

When the build is invoked with the installer product, the build SHALL force
installer media generation regardless of the board profile's default
`MAKE_INSTALLER` value, while still applying the profile's console and
target-disk defaults unless explicitly overridden. It SHALL NOT emit signed
RAUC update bundles unless bundle generation is explicitly requested. Staged
installer media SHALL be only the installer image this build produced.

#### Scenario: Profile installer off still emits media with installer product

- **WHEN** a board whose profile defaults `MAKE_INSTALLER` to off is built
  with the installer product
- **THEN** the staged artifacts include the installer medium

#### Scenario: Profile console still applies

- **WHEN** a board is built with the installer product and no console override
- **THEN** the installer and installed OS use the profile's console selection

#### Scenario: Installer product omits update bundles

- **WHEN** a board is built with the installer product and bundle generation
  is not explicitly requested
- **THEN** the staged artifacts do not include `.raucb` bundles

#### Scenario: Explicit bundle request still emits bundles

- **WHEN** a board is built with the installer product and bundle generation
  is explicitly requested
- **THEN** the staged artifacts include signed `.raucb` bundles

#### Scenario: Stale installer images are not staged

- **WHEN** the Buildroot images directory still holds installer images from an
  earlier console or build, and a board is built with the installer product
- **THEN** the staged artifacts include only the installer image produced by
  this build
