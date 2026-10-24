# Spec Delta

## Purpose

Joins the host to a RAVENNA network by publishing and receiving multicast
RTP streams on the existing PipeWire graph, using DNS-SD and RTSP, and
only while the PTP clock is locked.

## ADDED Requirements

### Requirement: Attaches to the existing PipeWire daemon

The daemon SHALL run as an `obake` user service and SHALL use that
user's PipeWire daemon. It SHALL NOT start a PipeWire daemon of its own.

#### Scenario: One PipeWire daemon

- **WHEN** the `obake-ravenna` user service is active
- **THEN** it uses the `obake` PipeWire socket
- **AND** no second PipeWire daemon is running

### Requirement: Stream configuration reloads without restarting PipeWire

The daemon SHALL read its sources and sinks from
`/home/obake/.config/obake-ravenna/`. Reloading the user service
SHALL apply that file and SHALL NOT restart PipeWire.

#### Scenario: Reload applies the edited file

- **WHEN** `obake` replaces the configuration and reloads the user
  service
- **THEN** the sources and sinks in that file are the ones in effect
- **AND** PipeWire was not restarted

### Requirement: An empty configuration is silent

When the configuration defines no sources and no sinks, the daemon
SHALL NOT transmit multicast and SHALL NOT join a multicast group.

#### Scenario: The seed transmits nothing

- **WHEN** the configuration defines no sources and no sinks and the
  service is active
- **THEN** the daemon sends no multicast audio and joins no multicast
  group

### Requirement: A source uses the graph rate and its configured frame size

A transmitting source SHALL use the live PipeWire graph sample rate.
Its format, channel count, frame size, and multicast destination SHALL
come from its configuration. The SDP it serves SHALL advertise that
rate, format, channel count, and destination, a packet time equal to
the frame size divided by the graph rate, and the local `ptp4l` domain.

#### Scenario: Served SDP matches the graph and the configuration

- **WHEN** a source is transmitting and a peer retrieves its SDP
- **THEN** the SDP sample rate equals the live graph rate
- **AND** the SDP format, channel count, group, and port equal the
  configuration
- **AND** the SDP packet time equals the configured frame size divided
  by the graph rate
- **AND** the SDP names the local `ptp4l` domain

### Requirement: Graph sample rate is an AES67 or RAVENNA media clock

The daemon SHALL treat a source or sink as down when the live PipeWire
graph rate is not one of 44100, 48000, 88200, 96000, 176400, 192000,
352800, or 384000 Hz.

#### Scenario: Unsupported graph rate stays down

- **WHEN** the graph rate is 32000 Hz
- **THEN** configured sources do not transmit

### Requirement: Source frame size is an allowed packet time

The daemon SHALL transmit a source only when its frame size at the live
graph rate is legal. On 48 kHz-family rates, the packet time SHALL be
one of 1/8, 1/4, 1/3, 2/3, 1, 4/3, or 4 milliseconds. On the 44.1 kHz
family, where those ptimes are not an integer number of samples, the
daemon SHALL accept the same absolute sample counts that are legal at
48000 Hz (6, 12, 16, 32, 48, 64, or 192).

#### Scenario: 48 samples at 48000 hertz transmits

- **WHEN** the graph rate is 48000 Hz, the frame size is 48 samples,
  the source is otherwise legal, and the PTP clock is locked
- **THEN** that source transmits

#### Scenario: 32 and 64 samples at 48000 hertz transmit

- **WHEN** the graph rate is 48000 Hz, the frame size is 32 or 64
  samples, the source is otherwise legal, and the PTP clock is locked
- **THEN** that source transmits

#### Scenario: 96 samples at 96000 hertz transmits

- **WHEN** the graph rate is 96000 Hz, the frame size is 96 samples,
  the source is otherwise legal, and the PTP clock is locked
- **THEN** that source transmits

#### Scenario: 48 samples at 44100 hertz transmits

- **WHEN** the graph rate is 44100 Hz, the frame size is 48 samples,
  the source is otherwise legal, and the PTP clock is locked
- **THEN** that source transmits

#### Scenario: 40 samples at 48000 hertz stays down

- **WHEN** the graph rate is 48000 Hz and the frame size is 40 samples
- **THEN** that source does not transmit

### Requirement: A source destination is scoped multicast

The daemon SHALL transmit a source only when its destination group is
inside `239.0.0.0/8` and is not `239.255.255.255`.

#### Scenario: The SAP group stays down

- **WHEN** a source destination group is `239.255.255.255`
- **THEN** that source does not transmit

#### Scenario: A unicast destination stays down

- **WHEN** a source destination is a unicast address
- **THEN** that source does not transmit

### Requirement: A source uses an even RTP port

The daemon SHALL transmit a source only on an even port. A destination
written without a port SHALL use port 5004. The daemon SHALL NOT send
that source's RTP to the following odd port.

#### Scenario: A bare group address uses port 5004

- **WHEN** a source destination is only a multicast group and the
  source is otherwise legal and transmitting
- **THEN** its RTP port is 5004

#### Scenario: An odd port stays down

- **WHEN** a source destination port is odd
- **THEN** that source does not transmit

### Requirement: Local sources do not share a destination

The daemon SHALL NOT transmit two configured sources to the same
multicast group and port.

#### Scenario: A duplicate destination stays down

- **WHEN** two configured sources name the same group and port
- **THEN** neither source transmits

### Requirement: Source packets leave on the PTP interface

Transmitted source packets SHALL leave on the interface named by the
`ptp4l` configuration. Their TTL SHALL be 15 unless that source sets
another TTL.

#### Scenario: Egress follows the ptp4l interface

- **WHEN** a source is transmitting
- **THEN** its multicast egress interface is the interface named in
  the `ptp4l` configuration

#### Scenario: Default TTL is 15

- **WHEN** a transmitting source sets no TTL
- **THEN** its multicast packets use TTL 15

### Requirement: A sink follows the named stream SDP

A configured sink SHALL select its remote stream by device name and
stream name. It SHALL take the multicast group, port, sample format,
channel count, sample rate, and frame size from that stream's SDP.

#### Scenario: The join address comes from the SDP

- **WHEN** the named device and stream are discovered and the SDP is
  otherwise acceptable
- **THEN** the daemon joins the SDP multicast group and port
- **AND** it receives with the SDP format, channel count, sample rate,
  and frame size

### Requirement: A sink accepts only a matching multicast SDP

The daemon SHALL join a sink only when the SDP connection address is
multicast, the SDP sample rate equals the live graph rate, and the SDP
PTP domain equals the local `ptp4l` domain.

#### Scenario: A unicast SDP stays down

- **WHEN** the named stream's SDP connection address is unicast
- **THEN** the sink does not join a group

#### Scenario: A rate mismatch stays down

- **WHEN** the SDP sample rate differs from the live graph rate
- **THEN** the sink does not join a group

#### Scenario: A different PTP domain stays down

- **WHEN** the SDP PTP domain differs from the local `ptp4l` domain
- **THEN** the sink does not join a group

### Requirement: PTP lock gates media

The daemon SHALL create transmitting and receiving PipeWire nodes only
while `ptp4l` reports a lock. Losing the lock SHALL remove those nodes.
A later lock SHALL create them again when the configuration and any
required SDP still qualify.

#### Scenario: An unlocked clock sends nothing

- **WHEN** `ptp4l` is inactive or reports no lock and a legal source
  is configured
- **THEN** that source does not transmit

#### Scenario: Losing the lock removes the nodes

- **WHEN** the PTP lock is lost while sources or sinks are active
- **THEN** those PipeWire RTP nodes are removed

#### Scenario: Regaining the lock restores qualifying streams

- **WHEN** the PTP lock returns and a source or sink still qualifies
- **THEN** its PipeWire RTP node is created again

### Requirement: A changed SDP replaces the sink

When a joined sink's SDP changes its group, port, format, sample rate,
or frame size, the daemon SHALL remove the previous receive node and
SHALL create a node for the new SDP only if that SDP still qualifies.

#### Scenario: A new group replaces the join

- **WHEN** a joined sink's SDP changes its multicast group to another
  multicast group and the new SDP still qualifies
- **THEN** the daemon leaves the previous group and joins the new one

### Requirement: The graph clock is unchanged

The daemon SHALL NOT change the PipeWire graph sample rate or quantum.

#### Scenario: Active streams leave the clock alone

- **WHEN** sources and sinks are active
- **THEN** the PipeWire graph sample rate and quantum are unchanged

### Requirement: The daemon does not discipline the clock

The daemon SHALL NOT step or slew the PTP hardware clock or the system
clock.

#### Scenario: Reading the lock does not adjust the clock

- **WHEN** the daemon reads the `ptp4l` lock state
- **THEN** it does not adjust the PTP hardware clock or the system
  clock

### Requirement: Reads PTP lock without spawning a process

The daemon SHALL read `ptp4l` lock state and domain over the read-only
management Unix socket using IEEE 1588 management messages in its own
process. It SHALL NOT spawn `pmc` or any other helper for that read.

#### Scenario: Lock is read with no child process

- **WHEN** the daemon queries whether the PTP clock is locked
- **THEN** it opens or uses the read-only management socket itself
- **AND** it does not create a child process for that query

#### Scenario: A missing socket holds streams unlocked

- **WHEN** the read-only management socket is absent or unreadable
- **THEN** the daemon treats the clock as unlocked
- **AND** configured sources and sinks stay down

### Requirement: RAVENNA services are browsed and published

The daemon SHALL browse and publish RAVENNA RTSP services through the
host mDNS service as the `obake` user. A peer SHALL be able to retrieve
SDP for a transmitting source by that source's configured name. A sink
SHALL match its configured device name to a browsed service name.

#### Scenario: A peer retrieves an active source

- **WHEN** a source is transmitting
- **THEN** a peer that resolves the published RTSP service and
  requests that source by name receives its SDP

#### Scenario: An unknown device stays down

- **WHEN** a sink names a device that is not among the browsed
  RAVENNA RTSP services
- **THEN** that sink does not join a group

### Requirement: Missing PTP interface holds streams down

When the `ptp4l` configuration does not name an interface that is
present, the daemon SHALL NOT transmit a source and SHALL NOT join a
sink.

#### Scenario: The placeholder interface sends nothing

- **WHEN** the `ptp4l` configuration still names its placeholder
  interface
- **THEN** no source transmits and no sink joins a group

### Requirement: Stream state can be queried

The `obake` user SHALL be able to query the running daemon for the PTP
lock state and, for each configured source and sink, whether it is
active and why it is held down.

#### Scenario: Status reports an unlocked source

- **WHEN** the PTP clock is unlocked and a source is configured
- **THEN** the query reports the clock unlocked and that source held
  down
