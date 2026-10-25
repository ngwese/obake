# Spec Delta

## ADDED Requirements

### Requirement: Network devices and sessions can be browsed

The `obake` user SHALL be able to run a one-shot `browse` command that
enumerates RAVENNA devices and sessions visible on the interface named
by the `ptp4l` configuration. The command SHALL NOT require the session
daemon to be running, and SHALL NOT change PipeWire objects, mDNS
registrations, or stream qualify state.

#### Scenario: Browse lists a peer device and its sessions

- **WHEN** a peer advertises a RAVENNA node and at least one session on
  the PTP interface and the operator runs `browse`
- **THEN** the report names that device
- **AND** lists each of that peer's sessions under it

#### Scenario: Browse works without the daemon

- **WHEN** the session daemon is not running and the operator runs
  `browse` on an interface with peer advertisements
- **THEN** the report still lists discovered devices and sessions

#### Scenario: Missing PTP interface yields no devices

- **WHEN** the `ptp4l` configuration does not name an interface that is
  present and the operator runs `browse`
- **THEN** the report lists no devices

### Requirement: Browse summarizes each session SDP

For each discovered session, `browse` SHALL DESCRIBE the session over
RTSP, parse the SDP, and print the sample rate, channel count, format,
frame size, and multicast group and port. When the session TXT includes
a `path` key, that path SHALL be used. When it does not, the command
SHALL DESCRIBE `/by-name/<session instance name>`. A DESCRIBE or SDP
failure SHALL still list the session with a short failure reason.

#### Scenario: Session summary from a successful DESCRIBE

- **WHEN** a discovered session answers DESCRIBE with a parseable SDP
- **THEN** the report shows that session's sample rate, channel count,
  format, frame size, and multicast group and port

#### Scenario: Missing TXT path uses by-name

- **WHEN** a discovered session has no TXT `path` and its instance name
  is `PGM-1`
- **THEN** the command DESCRIBEs `/by-name/PGM-1` on that session's
  resolved host and port

#### Scenario: Failed DESCRIBE still lists the session

- **WHEN** a discovered session's DESCRIBE fails or the SDP cannot be
  parsed
- **THEN** the report still names that session under its device
- **AND** includes a short reason instead of the media summary
