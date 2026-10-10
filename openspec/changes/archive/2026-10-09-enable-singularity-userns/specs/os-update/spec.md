# Spec Delta

## MODIFIED Requirements

### Requirement: USB and HTTPS delivery

The system SHALL be able to obtain update bundles from a configured signed
HTTPS endpoint and from removable USB media, using the same bundle format and
the same signature verification for both. When a bundle is present on
auto-mounted removable media under `/media` the system SHALL prefer that USB
source over HTTPS. The update client SHALL also accept a caller-supplied local
bundle path on `apply`, with the same signature verification as other sources.
The `obake` user SHALL be able to request apply/rollback without unlocking the
root account, using the host's limited sudo grant for `obake-update`.

#### Scenario: Airgapped update from USB

- **WHEN** a signed bundle is present on auto-mounted removable media under
  `/media` and the host has no network access
- **THEN** the system installs the update from that USB bundle

#### Scenario: USB preferred when both available

- **WHEN** a valid bundle is present on auto-mounted removable media under
  `/media` and a newer bundle is also reachable over HTTPS
- **THEN** the system uses the USB bundle in preference to the HTTPS source

#### Scenario: Apply as obake without root login

- **WHEN** the `obake` user runs `obake-update apply` with a valid bundle
  available
- **THEN** the install proceeds without logging in as root and without an
  interactive password prompt

#### Scenario: Apply a local bundle path as obake

- **WHEN** the `obake` user runs `obake-update apply` with a path to a signed
  local `.raucb` file
- **THEN** the system installs that bundle using the same signature check as
  USB and HTTPS sources, without a root login or interactive password prompt

#### Scenario: Reject a local invalid bundle as obake

- **WHEN** the `obake` user runs `obake-update apply` with a path to a
  tampered or untrusted local `.raucb` file
- **THEN** the system rejects it, reports the failure, and leaves all slots
  unchanged
