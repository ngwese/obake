# Spec Delta

## ADDED Requirements

### Requirement: Board build profiles

Each board SHALL provide a `default` and a `virtual` build profile. A profile
supplies the board's installer build defaults (`MAKE_INSTALLER`,
`INSTALLER_CONSOLE`, `INSTALLER_TARGET_DISK`); `default` targets the physical
board and `virtual` targets an emulation or verification target, and the generic
boards' two profiles are identical. When no profile is named the build SHALL use
`default`, an explicit setting SHALL override the profile value, and an unknown
profile SHALL fail the build.

#### Scenario: Default profile selected

- **WHEN** a board is built without naming a profile
- **THEN** it applies the `default` profile's installer defaults

#### Scenario: Virtual profile selected

- **WHEN** a board is built naming the `virtual` profile
- **THEN** it applies that profile's emulation-oriented installer defaults

#### Scenario: Explicit setting overrides the profile

- **WHEN** an installer setting is also provided explicitly with a profile
- **THEN** the explicit value takes precedence over the profile value

#### Scenario: Generic profiles are equivalent

- **WHEN** a generic board is built with either profile
- **THEN** the resulting build configuration is the same

#### Scenario: Unknown profile rejected

- **WHEN** a build names a profile that the board does not provide
- **THEN** the build fails with an error naming the supported profiles

### Requirement: Board-implied target architecture

Building a board SHALL derive the target architecture from the board's
configuration, so the operator does not supply an architecture separately.

#### Scenario: Architecture follows the board

- **WHEN** a board is built without an architecture argument
- **THEN** the emitted image targets the architecture declared by that board

#### Scenario: Distinct architectures from distinct boards

- **WHEN** a board that targets x86_64 and a board that targets aarch64 are
  built the same way
- **THEN** each emitted image targets its own board's architecture
