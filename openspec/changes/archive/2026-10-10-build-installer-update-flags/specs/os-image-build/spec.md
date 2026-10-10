# Spec Delta

## ADDED Requirements

### Requirement: Installer product emits installer media

When the build is invoked with the installer product, the build SHALL force
installer media generation regardless of the board profile's default
`MAKE_INSTALLER` value, while still applying the profile's console and
target-disk defaults unless explicitly overridden.

#### Scenario: Profile installer off still emits media with installer product

- **WHEN** a board whose profile defaults `MAKE_INSTALLER` to off is built
  with the installer product
- **THEN** the staged artifacts include the installer medium

#### Scenario: Profile console still applies

- **WHEN** a board is built with the installer product and no console override
- **THEN** the installer and installed OS use the profile's console selection

### Requirement: Update product emits signed bundles only

When the build is invoked with the update product, the build SHALL disable
installer media and full bootable disk image assembly by default, and SHALL
emit signed RAUC update bundles for both slots (unless a slot filter is set)
using the effective signing material.

#### Scenario: Update product skips installer and disk image

- **WHEN** a board is built with the update product and no explicit disk or
  installer overrides
- **THEN** the staged artifacts include signed `.raucb` bundles and do not
  include installer media or `disk-*.img`

#### Scenario: Explicit override can re-enable disk or installer

- **WHEN** a board is built with the update product and an explicit environment
  override re-enables disk or installer assembly
- **THEN** the override takes effect
