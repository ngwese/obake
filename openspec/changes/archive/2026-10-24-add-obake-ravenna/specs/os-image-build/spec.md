# Spec Delta

## ADDED Requirements

### Requirement: qdl image contains obake-ravenna

The `qdl` image SHALL include the `obake-ravenna` program. Every
other board image SHALL omit that program.

#### Scenario: qdl rootfs contains the daemon

- **WHEN** a `qdl` image is built
- **THEN** the root filesystem provides `obake-ravenna` as an
  executable command

#### Scenario: Other boards omit the daemon

- **WHEN** an image is built for a board other than `qdl`
- **THEN** the root filesystem does not provide `obake-ravenna`

### Requirement: obake-ravenna is an enabled obake user service

The `qdl` image SHALL enable `obake-ravenna` as a user service of
the `obake` account so the service is active after boot without an
interactive login. It SHALL run in the `obake` user manager. It SHALL
NOT be installed as a system service.

#### Scenario: The user service is active after boot

- **WHEN** a `qdl` host has finished boot and no one has logged in
- **THEN** `obake-ravenna` is active in the `obake` user manager

#### Scenario: It is not a system service

- **WHEN** the system service manager on a `qdl` image is inspected
- **THEN** `obake-ravenna` is not installed as a system service

### Requirement: obake-ravenna configuration is seeded once

The `qdl` image SHALL create
`/home/obake/.config/obake-ravenna/` only when that configuration
is missing. The created file SHALL be owned by `obake` and SHALL
define no sources and no sinks. An existing configuration SHALL be
left unchanged across boot and image update.

#### Scenario: First boot seeds an empty stream list

- **WHEN** the configuration is absent and a `qdl` host boots
- **THEN** the configuration exists, is owned by `obake`, and defines
  no sources and no sinks

#### Scenario: An existing configuration is kept

- **WHEN** `/home/obake/.config/obake-ravenna/` already contains a
  configuration and the host boots or applies an image update
- **THEN** that configuration is unchanged

### Requirement: seeded ptp4l exposes a management socket to obake

The seeded `ptp4l` configuration SHALL place its management socket on
a path the `obake` user can open. The `ptp4l` service SHALL create the
directory for that socket as `obake`. This SHALL NOT enable `ptp4l`
at boot.

#### Scenario: A started seed is readable by obake

- **WHEN** `ptp4l` is started from a configuration the seed created
- **THEN** uid `obake` can open the management socket

#### Scenario: The socket change does not enable ptp4l

- **WHEN** a `qdl` host boots an image on which the operator has not
  enabled `ptp4l`
- **THEN** `ptp4l` is inactive
