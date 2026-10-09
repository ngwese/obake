# Spec Delta

## ADDED Requirements

### Requirement: Profile console for installer and installed OS

The build-time `CONSOLE` setting (`serial` or `local`) SHALL select the
interactive console for both the installer medium and the installed OS
unified kernel images, including the kernel command line and getty, so a
board profile's console choice is consistent from install through first boot.

#### Scenario: Local console on installer and installed OS

- **WHEN** a board is built with `CONSOLE=local`
- **THEN** the installer medium and each installed OS UKI present on the local
  text console

#### Scenario: Serial console on installer and installed OS

- **WHEN** a board is built with `CONSOLE=serial`
- **THEN** the installer medium and each installed OS UKI present on the
  board's serial console

#### Scenario: qdl default uses local end-to-end

- **WHEN** the qdl board is built with its default profile
- **THEN** both the installer medium and the installed OS UKIs use the local
  text console

## MODIFIED Requirements

### Requirement: Board build profiles

Each board SHALL provide a `default` and a `virtual` build profile. A profile
supplies the board's installer and console build defaults (`MAKE_INSTALLER`,
`CONSOLE`, `INSTALLER_TARGET_DISK`); `default` targets the physical board and
`virtual` targets an emulation or verification target, and the generic
boards' two profiles are identical. When no profile is named the build SHALL
use `default`, an explicit setting SHALL override the profile value, and an
unknown profile SHALL fail the build.

#### Scenario: Default profile selected

- **WHEN** a board is built without naming a profile
- **THEN** it applies the `default` profile's installer and console defaults

#### Scenario: Virtual profile selected

- **WHEN** a board is built naming the `virtual` profile
- **THEN** it applies that profile's emulation-oriented installer and console
  defaults

#### Scenario: Explicit setting overrides the profile

- **WHEN** an installer or console setting is also provided explicitly with a
  profile
- **THEN** the explicit value takes precedence over the profile value

#### Scenario: Generic profiles are equivalent

- **WHEN** a generic board is built with either profile
- **THEN** the resulting build configuration is the same

#### Scenario: Unknown profile rejected

- **WHEN** a build names a profile that the board does not provide
- **THEN** the build fails with an error naming the supported profiles
