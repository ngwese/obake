# Spec Delta

## MODIFIED Requirements

### Requirement: seeded qdl PTP configuration is an AES67 client

The seeded `ptp4l` configuration SHALL be a client-only AES67/RAVENNA
profile: domain 0, end-to-end delay, UDPv4, and hardware timestamping.
It SHALL name no interface the image creates. It SHALL set a non-zero
`step_threshold` so the PHC may jump when the measured offset exceeds
that threshold after start (not only on the first servo update). The
seeded `phc2sys` options SHALL slew the system clock and SHALL NOT step
it.

#### Scenario: Seed is a client with no real interface

- **WHEN** the seed creates `/home/obake/.config/linuxptp/ptp4l.cfg`
- **THEN** the file selects client-only operation, domain 0, end-to-end
  delay, UDPv4, and hardware timestamping
- **AND** it does not name an interface the image creates

#### Scenario: Seed steps the PHC on large offset

- **WHEN** the seed creates `/home/obake/.config/linuxptp/ptp4l.cfg`
- **THEN** the file sets a non-zero `step_threshold`

#### Scenario: Seed does not step the system clock

- **WHEN** the seed creates the `phc2sys` options file
- **THEN** those options slew the system clock
- **AND** they do not request a step of the system clock
