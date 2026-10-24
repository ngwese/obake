# Spec Delta

## Purpose

obake-ravenna manages one RAVENNA graph on the host PipeWire daemon:
a PTP driver plus the configured transmit and receive streams, with SDP
and DNS-SD that peers can lock to.

## ADDED Requirements

### Requirement: Attaches to the existing PipeWire daemon

The daemon SHALL run as an `obake` user service and SHALL use that
user's PipeWire daemon. It SHALL NOT start a PipeWire daemon of its own.

#### Scenario: One PipeWire daemon

- **WHEN** the `obake-ravenna` user service is active
- **THEN** it uses the `obake` PipeWire socket
- **AND** no second PipeWire daemon is running

### Requirement: Configuration reloads without restarting PipeWire

The daemon SHALL read graph, multicast, source, and sink settings from
`/home/obake/.config/obake-ravenna/`. Reloading the user service SHALL
apply that file and SHALL NOT restart PipeWire.

#### Scenario: Reload applies the edited file

- **WHEN** `obake` replaces the configuration and reloads the user
  service
- **THEN** the graph settings and streams in that file are the ones in
  effect
- **AND** PipeWire was not restarted

### Requirement: An empty configuration transmits nothing

When the configuration defines no sources and no sinks, the daemon
SHALL NOT transmit multicast audio and SHALL NOT join a multicast group.

#### Scenario: The seed transmits nothing

- **WHEN** the configuration defines no sources and no sinks and the
  service is active
- **THEN** the daemon sends no multicast audio and joins no multicast
  group

### Requirement: Media rate is not the graph quantum

The configured sample rate SHALL be the rate of every RAVENNA object
the daemon creates. The daemon SHALL NOT change PipeWire's default
clock rate or quantum, and SHALL NOT set that quantum from the RTP
frames-per-packet setting.

#### Scenario: Packet size leaves the quantum alone

- **WHEN** the configured rate is 48000 Hz, frames per packet is 48,
  and the PipeWire quantum is 128 samples
- **THEN** a transmitting source's RTP packet time is 1 ms
- **AND** the PipeWire quantum is still 128 samples

### Requirement: Packet size is independent of the quantum

RTP frames per packet SHALL be a daemon setting separate from the
PipeWire quantum. The transmitter SHALL emit packets of that size
while the quantum differs, retaining samples between graph cycles
instead of waiting until one cycle equals one packet.

#### Scenario: A 128-sample cycle still emits 48-frame packets

- **WHEN** the quantum is 128 samples, frames per packet is 48, and a
  source is transmitting
- **THEN** each RTP packet carries 48 frames
- **AND** the daemon does not require three graph cycles before the
  first packet

### Requirement: Graph sample rate is an AES67 or RAVENNA media clock

The daemon SHALL treat every source and sink as down when the
configured rate is not one of 44100, 48000, 88200, 96000, 176400,
192000, 352800, or 384000 Hz.

#### Scenario: Unsupported rate stays down

- **WHEN** the configured rate is 32000 Hz
- **THEN** configured sources do not transmit

### Requirement: Frames per packet is an allowed packet time

The daemon SHALL transmit only when the configured frames per packet
at the configured rate is an AES67 packet time. On 48 kHz-family rates
that time SHALL be one of 1/8, 1/4, 1/3, 2/3, 1, 4/3, or 4
milliseconds. On the 44.1 kHz family the daemon SHALL accept the same
absolute sample counts that are legal at 48000 Hz (6, 12, 16, 32, 48,
64, or 192).

#### Scenario: 48 frames at 48000 hertz transmits

- **WHEN** the rate is 48000 Hz, frames per packet is 48, and the
  source is otherwise legal and the PTP clock is locked
- **THEN** that source transmits

#### Scenario: 40 frames at 48000 hertz stays down

- **WHEN** the rate is 48000 Hz and frames per packet is 40
- **THEN** no source transmits

### Requirement: Transmit address is one scoped multicast base

The transmit multicast address SHALL come from the daemon multicast
settings. The daemon SHALL transmit only when that address is inside
`239.0.0.0/8` and is not `239.255.255.255`. The daemon SHALL NOT assign
a different address to each source by incrementing the base.

#### Scenario: The SAP group stays down

- **WHEN** the multicast base address is `239.255.255.255`
- **THEN** no source transmits

#### Scenario: Sources share the base address

- **WHEN** two sources are transmitting and neither sets its own
  address
- **THEN** both use the configured base multicast address

### Requirement: Transmit ports step from an even base

The daemon SHALL assign each source an even RTP port of
`base_port + 2 * index`, where `index` is that source's position among
configured sources starting at zero. `base_port` SHALL be even. The
daemon SHALL NOT send that source's RTP to the following odd port.

#### Scenario: The first two sources use consecutive even ports

- **WHEN** `base_port` is 5004 and the first two sources are
  transmitting
- **THEN** their RTP ports are 5004 and 5006

#### Scenario: An odd base port stays down

- **WHEN** `base_port` is odd
- **THEN** no source transmits

### Requirement: A source may pin address or port

A source MAY set its own multicast address or port. A pinned port SHALL
be even. Two sources SHALL NOT transmit to the same address and port.

#### Scenario: A duplicate destination stays down

- **WHEN** two sources resolve to the same multicast address and port
- **THEN** neither source transmits

### Requirement: A packet payload fits without fragmentation

Audio payload bytes SHALL be channels times frames per packet times
bytes per sample, with L24 counting as 3 bytes. The daemon SHALL NOT
transmit when that payload exceeds 1440 bytes. It SHALL NOT raise the
interface MTU and SHALL NOT rely on IP fragmentation.

#### Scenario: Eight-channel L24 at 64 frames per packet stays down

- **WHEN** a source is 8 channels of L24 and frames per packet is 64
- **THEN** that source does not transmit

#### Scenario: Eight-channel L24 at 48 frames per packet transmits

- **WHEN** a source is 8 channels of L24, frames per packet is 48,
  and the source is otherwise legal
- **THEN** that source may transmit

### Requirement: Source packets leave on the PTP interface

Transmitted packets SHALL leave on the interface named by the `ptp4l`
configuration. TTL SHALL be the configured multicast TTL, defaulting
to 15 when unset.

#### Scenario: Egress follows the ptp4l interface

- **WHEN** a source is transmitting
- **THEN** its multicast egress interface is the interface named in
  the `ptp4l` configuration

### Requirement: SDP names the grandmaster clock

A transmitting source's SDP SHALL advertise the configured rate,
format, channel count, address, and port, a packet time equal to the
frames per packet divided by the rate, and `a=ts-refclk` naming the current
`ptp4l` grandmaster clock identity and domain. The daemon SHALL NOT
advertise an all-zero clock identity.

#### Scenario: A locked grandmaster appears in the SDP

- **WHEN** a source is transmitting and a peer retrieves its SDP
- **THEN** `a=ts-refclk` contains that grandmaster's clock identity
  and the local `ptp4l` domain
- **AND** the identity is not all zeros

#### Scenario: No grandmaster holds the source down

- **WHEN** `ptp4l` reports no grandmaster clock identity
- **THEN** that source does not transmit

### Requirement: Payload type follows channel count

A transmitting source's RTP payload type SHALL be `96` plus its
channel count, unless that source sets a payload type. The served SDP
and the transmitted RTP payload type SHALL be the same value, in the
range 96 through 127.

#### Scenario: Two channels use payload type 98

- **WHEN** a transmitting source has 2 channels and sets no payload
  type
- **THEN** its SDP and RTP payload type are 98

#### Scenario: An out-of-range payload type stays down

- **WHEN** a source's payload type is below 96 or above 127
- **THEN** that source does not transmit

### Requirement: One PTP driver for the RAVENNA group

While the `ptp4l` interface is present, the daemon SHALL ensure one
PTP graph driver whose clock follows that interface and whose node
group is the configured RAVENNA group. RTP nodes the daemon creates
SHALL use that same group and SHALL NOT use a separate AES67 driver
group. The driver SHALL NOT be exported as a second node.

#### Scenario: Active streams share the driver group

- **WHEN** a source is transmitting
- **THEN** its RTP node and the PTP driver share the configured node
  group
- **AND** exactly one such driver node exists

#### Scenario: Shutdown removes the driver

- **WHEN** the daemon stops
- **THEN** the PTP driver it created is removed

### Requirement: Stream nodes follow qualify and config

The daemon SHALL create an RTP transmit node only for a source that
qualifies, and an RTP receive node only for a sink that qualifies. It
SHALL update a node when its arguments change and SHALL remove it when
the stream is removed, fails to qualify, or the daemon stops.

#### Scenario: Removing a source removes its node

- **WHEN** a transmitting source is removed from the configuration and
  the service reloads
- **THEN** that transmit node is gone
- **AND** PipeWire was not restarted

#### Scenario: A changed argument replaces the node

- **WHEN** a transmitting source's channel count changes and it still
  qualifies
- **THEN** the previous transmit node is removed and a node with the
  new channel count is created

### Requirement: PipeWire object changes are logged

The daemon SHALL log each create, update, and delete of a PipeWire
object at info, including the object name. At debug it SHALL log the
configuration applied to that object.

#### Scenario: Creating a transmit node is logged

- **WHEN** the daemon creates a transmit node
- **THEN** an info log names that object as created

### Requirement: A sink follows the named stream SDP

A configured sink SHALL select its remote stream by device name and
stream name. It SHALL take the multicast group, port, sample format,
channel count, sample rate, frame size, and payload type from that
stream's SDP.

#### Scenario: The join address comes from the SDP

- **WHEN** the named device and stream are discovered and the SDP is
  otherwise acceptable
- **THEN** the daemon joins the SDP multicast group and port
- **AND** it receives with the SDP format, channel count, sample rate,
  and frame size

### Requirement: A sink accepts only a matching multicast SDP

The daemon SHALL join a sink only when the SDP connection address is
multicast, the SDP sample rate equals the configured graph rate, and
the SDP PTP domain equals the local `ptp4l` domain.

#### Scenario: A rate mismatch stays down

- **WHEN** the SDP sample rate differs from the configured graph rate
- **THEN** the sink does not join a group

#### Scenario: A different PTP domain stays down

- **WHEN** the SDP PTP domain differs from the local `ptp4l` domain
- **THEN** the sink does not join a group

### Requirement: PTP lock gates media nodes

The daemon SHALL create transmitting and receiving nodes only while
`ptp4l` reports a lock and a grandmaster identity. Losing the lock
SHALL remove those nodes. A later lock SHALL create them again when
they still qualify. The daemon SHALL NOT step or slew any clock.

#### Scenario: Losing the lock removes the stream nodes

- **WHEN** the PTP lock is lost while sources or sinks are active
- **THEN** those RTP nodes are removed

### Requirement: Reads PTP state without spawning a process

The daemon SHALL read `ptp4l` lock state, domain, and grandmaster
clock identity over the read-only management Unix socket in its own
process. It SHALL NOT spawn `pmc` or any other helper for that read.
A missing socket SHALL be treated as unlocked.

#### Scenario: Lock is read with no child process

- **WHEN** the daemon queries the PTP clock
- **THEN** it uses the read-only management socket itself
- **AND** it does not create a child process for that query

### Requirement: Sessions publish an RTSP path

The daemon SHALL publish the node and each transmitting session as
RAVENNA RTSP services on the `ptp4l` interface. Each session record
SHALL include a TXT `path` of `/by-name/<stream>`. A peer that
DESCRIBEs that path SHALL receive the source SDP. A sink SHALL match
its device name to a browsed node service name.

#### Scenario: A peer retrieves an active source

- **WHEN** a source is transmitting and a peer DESCRIBEs
  `/by-name/<that source>` on the published RTSP service
- **THEN** the response body is that source's SDP

#### Scenario: The session TXT names the path

- **WHEN** a source is transmitting
- **THEN** its session DNS-SD record contains
  `path=/by-name/<that source>`

### Requirement: Missing PTP interface holds streams down

When the `ptp4l` configuration does not name an interface that is
present, the daemon SHALL NOT create the PTP driver, SHALL NOT
transmit, and SHALL NOT join a sink.

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
