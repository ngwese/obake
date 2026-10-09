# Spec Delta

## MODIFIED Requirements

### Requirement: Headless installer

The installer SHALL operate over a serial console or a text console with an
attached keyboard and monitor, SHALL support selecting which console the
installer medium uses at build time, and SHALL NOT require a graphical
interface at any point.

#### Scenario: Install on a headless host

- **WHEN** the installer runs on a host with no display server
- **THEN** all prompts, selections, and progress are available over the
  selected serial or text console

#### Scenario: Install over serial

- **WHEN** the installer medium is built with the serial console selected
- **THEN** all prompts, selections, and progress are available over the serial
  console

#### Scenario: Install over a local text console

- **WHEN** the installer medium is built with the local console selected and
  runs on a host whose firmware provides a framebuffer
- **THEN** all prompts, selections, and progress are available on the attached
  monitor and the attached keyboard drives the installer

## ADDED Requirements

### Requirement: Installer console selection

The installer build SHALL select the medium's interactive console between
serial and local, SHALL default to serial so headless CI and QEMU verification
are unchanged, and SHALL reject an unrecognized selection.

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
