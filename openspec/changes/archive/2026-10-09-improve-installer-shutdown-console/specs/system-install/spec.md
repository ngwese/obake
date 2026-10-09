# Spec Delta

## ADDED Requirements

### Requirement: Post-install shutdown wait

After the installer finishes (success or failure), it SHALL wait before
powering off the host: when built for interactive target-disk selection it
SHALL prompt on the selected console for operator confirmation; when built
non-interactive it SHALL wait a build-time duration that defaults to 10
seconds and MAY be set to zero for automated tooling.

#### Scenario: Interactive confirm before poweroff

- **WHEN** the installer medium was built for interactive disk selection and
  the install script finishes
- **THEN** the installer prompts on the selected console for confirmation and
  powers off only after the operator confirms

#### Scenario: Non-interactive default delay

- **WHEN** the installer medium was built with a fixed target disk and no
  custom poweroff delay, and the install script finishes
- **THEN** the installer waits 10 seconds on the selected console and then
  powers off

#### Scenario: Non-interactive zero delay for automation

- **WHEN** the installer medium was built with a poweroff delay of zero and
  the install script finishes
- **THEN** the installer powers off without waiting

## MODIFIED Requirements

### Requirement: Installer console selection

The installer build SHALL select the medium's interactive console between
serial and local via the build-time `CONSOLE` setting, SHALL default to
serial so headless CI and QEMU verification are unchanged, and SHALL reject
an unrecognized selection.

#### Scenario: Default selection is serial

- **WHEN** an installer medium is built without specifying a console
- **THEN** it presents the installer over the serial console

#### Scenario: Local console selected

- **WHEN** an installer medium is built with the local console selection
- **THEN** it presents the installer on a local virtual terminal and accepts
  input from an attached USB keyboard

#### Scenario: Unrecognized selection rejected

- **WHEN** an installer medium is built with an unrecognized console selection
- **THEN** the build fails with an error naming the supported selections
