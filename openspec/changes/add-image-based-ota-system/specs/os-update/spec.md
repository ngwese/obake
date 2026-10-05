# Spec Delta

## Purpose

Delivers user-initiated operating system updates to an obake host in a way that
survives power loss, carries new kernels, works on airgapped networks, and
automatically reverts a failed update so a host is never left unbootable.

## ADDED Requirements

### Requirement: User-initiated updates only

The system SHALL apply an update only after an explicit user request and SHALL
NOT apply updates automatically in the background.

#### Scenario: No update without a request

- **WHEN** a newer update is available and the user has not requested an update
- **THEN** the running system and its active slot remain unchanged

#### Scenario: Update proceeds on request

- **WHEN** the user requests an update
- **THEN** the system begins applying the newest available update

### Requirement: Signed update bundles

The system SHALL accept update bundles only when they carry a valid signature
matching the project's trusted signing material, and SHALL reject any bundle
that fails verification without modifying a slot.

#### Scenario: Valid signature accepted

- **WHEN** a bundle with a valid signature is presented
- **THEN** the system proceeds to install it

#### Scenario: Invalid signature rejected

- **WHEN** a bundle with a missing, malformed, or untrusted signature is
  presented
- **THEN** the system rejects it, reports the failure, and leaves all slots
  unchanged

### Requirement: USB and HTTPS delivery

The system SHALL be able to obtain update bundles from a configured signed
HTTPS endpoint and from removable USB media, using the same bundle format and
the same signature verification for both. When a bundle is present on USB media
the system SHALL prefer the USB source over HTTPS.

#### Scenario: Airgapped update from USB

- **WHEN** a signed bundle is present on removable USB media and the host has no
  network access
- **THEN** the system installs the update from the USB bundle

#### Scenario: USB preferred when both available

- **WHEN** a valid bundle is present on USB media and a newer bundle is also
  reachable over HTTPS
- **THEN** the system uses the USB bundle in preference to the HTTPS source

### Requirement: A/B slot application with kernel

The system SHALL write an update to the inactive root slot together with the
matching per-slot unified kernel image, and SHALL activate the updated slot on
the next boot without modifying the currently active slot.

#### Scenario: Inactive slot written

- **WHEN** an update is applied
- **THEN** only the inactive slot and its unified kernel image are written

#### Scenario: Kernel updates with userspace

- **WHEN** a bundle contains a new kernel
- **THEN** the new kernel is installed as part of the same slot activation so
  kernel and userspace change together

### Requirement: Health gate and automatic rollback

The system SHALL run a health-gate service on boot that restores the prior user
configuration and validates the running update over a bounded number of boots.
If the update is not marked successful within that bound, the system SHALL
automatically revert to the previous slot.

#### Scenario: Successful update marked good

- **WHEN** the health-gate service validates the running update and marks it
  successful
- **THEN** the updated slot remains the active slot on subsequent boots

#### Scenario: Failed update reverts

- **WHEN** the update is not marked successful within the bounded number of
  boots
- **THEN** the system automatically boots the previous slot and restores the
  prior user configuration

### Requirement: Update status visibility

The system SHALL allow a user to query the currently running version, the
active slot, and the state of any pending or failed update.

#### Scenario: User queries status

- **WHEN** the user requests update status
- **THEN** the system reports the running version, the active slot, and any
  pending or failed update state
