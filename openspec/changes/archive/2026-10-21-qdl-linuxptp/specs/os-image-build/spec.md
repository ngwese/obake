# Spec Delta

## ADDED Requirements

### Requirement: qdl image contains the linuxptp suite

The `qdl` image SHALL include the linuxptp 4.4 commands `ptp4l`,
`phc2sys`, `pmc`, `phc_ctl`, `hwstamp_ctl`, `timemaster`, `ts2phc`,
`nsm`, and `tz2alt`. Every other board image SHALL omit that suite.

#### Scenario: qdl rootfs contains the suite

- **WHEN** a `qdl` image is built
- **THEN** the root filesystem provides `ptp4l`, `phc2sys`, `pmc`,
  `phc_ctl`, `hwstamp_ctl`, `timemaster`, `ts2phc`, `nsm`, and `tz2alt`
  as executable commands

#### Scenario: Other boards omit the suite

- **WHEN** an image is built for a board other than `qdl`
- **THEN** the root filesystem does not provide `ptp4l`

### Requirement: qdl PTP services stay stopped until enabled

The `qdl` image SHALL provide `ptp4l` and `phc2sys` as system services
that are not enabled. An unmodified boot SHALL leave both stopped.
Starting or enabling `ptp4l` SHALL NOT start `phc2sys`.

#### Scenario: Fresh boot leaves both stopped

- **WHEN** a `qdl` host boots an image on which the operator has not
  enabled `ptp4l` or `phc2sys`
- **THEN** both services are inactive

#### Scenario: ptp4l does not start phc2sys

- **WHEN** the operator starts `ptp4l` and has not enabled `phc2sys`
- **THEN** `phc2sys` remains inactive

#### Scenario: Operator enables ptp4l

- **WHEN** the `obake` user enables and starts `ptp4l` over SSH
- **THEN** the action succeeds without a password prompt
- **AND** `journalctl -u ptp4l` shows that service's log

### Requirement: qdl PTP services run as the obake user

Started `ptp4l` and `phc2sys` services SHALL run as the `obake` user.
The running `ptp4l` service SHALL be able to bind the PTP event and
general ports and open the board PHC. The running `phc2sys` service
SHALL be able to discipline the system clock. Executing those binaries
from an `obake` login shell SHALL NOT grant that shell the same
privileges.

#### Scenario: Service identity is obake

- **WHEN** `ptp4l` is running as a system service
- **THEN** its process uid is `obake`
- **AND** it is not a service of the `obake` user manager

#### Scenario: Login shell does not inherit service privileges

- **WHEN** the `obake` user runs `ptp4l` from a login shell against the
  seeded configuration
- **THEN** the process cannot bind the PTP event port

### Requirement: obake can edit the qdl PTP configuration

The `ptp4l` service SHALL read its configuration from
`/home/obake/.config/linuxptp/`, and `phc2sys` SHALL read its options
from that same directory. The `obake` user SHALL be able to modify both
files. The image SHALL create a file there only when it is missing, and
SHALL leave an existing file unchanged across boot and image update.

#### Scenario: obake edits the ptp4l configuration

- **WHEN** the `obake` user changes
  `/home/obake/.config/linuxptp/ptp4l.cfg`
- **THEN** the write succeeds
- **AND** a later start of the `ptp4l` service uses that file

#### Scenario: Existing configuration is kept

- **WHEN** `/home/obake/.config/linuxptp/ptp4l.cfg` already exists and
  the host boots or applies an image update
- **THEN** the file contents are unchanged

### Requirement: seeded qdl PTP configuration is an AES67 client

The seeded `ptp4l` configuration SHALL be a client-only AES67/RAVENNA
profile: domain 0, end-to-end delay, UDPv4, and hardware timestamping.
It SHALL name no interface the image creates. The seeded `phc2sys`
options SHALL slew the system clock and SHALL NOT step it.

#### Scenario: Seed is a client with no real interface

- **WHEN** the seed creates `/home/obake/.config/linuxptp/ptp4l.cfg`
- **THEN** the file selects client-only operation, domain 0, end-to-end
  delay, UDPv4, and hardware timestamping
- **AND** it does not name an interface the image creates

#### Scenario: Seed does not step the system clock

- **WHEN** the seed creates the `phc2sys` options file
- **THEN** those options slew the system clock
- **AND** they do not request a step of the system clock
