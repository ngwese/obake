# Spec Delta

## ADDED Requirements

### Requirement: Runtime layer contains obake-session

Every board image that selects the obake runtime layer SHALL include the
`obake-session` service and the `obake` program as executable
commands.

#### Scenario: A runtime image contains the command

- **WHEN** a board image with the obake runtime layer is built
- **THEN** the root filesystem provides `obake-session` and `obake` as
  executable commands

### Requirement: obake-session is an enabled obake user service

The runtime layer SHALL enable `obake-session` as a user service of the
`obake` account so the service is active after boot without an
interactive login. It SHALL run in the `obake` user manager. It SHALL
NOT be installed as a system service.

#### Scenario: The user service is active after boot

- **WHEN** a host with the runtime layer has finished boot and no one has
  logged in
- **THEN** `obake-session` is active in the `obake` user manager

#### Scenario: It is not a system service

- **WHEN** the system service manager on that image is inspected
- **THEN** `obake-session` is not installed as a system service

### Requirement: Existing session state survives image update

An image update SHALL NOT modify an existing
`/home/obake/.config/obake-session/` or an existing session-state
directory under the `obake` home directory. The image SHALL create those
directories, owned by `obake`, only when they are missing, and the
created rig list SHALL be empty.

#### Scenario: First boot creates an empty rig list

- **WHEN** the session configuration is absent and the host boots
- **THEN** the configuration exists, is owned by `obake`, and lists no
  rigs

#### Scenario: An update keeps installed rigs

- **WHEN** `/home/obake/.config/obake-session/` already contains an
  rig and the host applies an image update
- **THEN** that rig is unchanged
