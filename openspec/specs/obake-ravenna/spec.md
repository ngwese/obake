# obake-ravenna Specification

## Purpose

obake-ravenna manages one RAVENNA graph on the host PipeWire daemon:
a PTP driver plus the configured transmit and receive streams, with SDP
and DNS-SD that peers can lock to.

## Requirements

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

### Requirement: Multicast allocation mode selects port or merging layout

The daemon multicast settings SHALL accept an allocation mode of `port`
or `merging`. When the mode is omitted, the daemon SHALL use `port`.
Mode `port` is the existing shared-group, stepped-port layout. Mode
`merging` mimics observed Merging device behavior: a shared even RTP
port and a distinct group per source. A generic address-stepping mode
SHALL NOT be introduced by this change.

#### Scenario: Omitted allocation keeps port mode

- **WHEN** `[multicast]` omits `allocation`
- **AND** two sources transmit without pinned addresses or ports
- **THEN** both use the configured base multicast address
- **AND** their RTP ports are `base_port` and `base_port + 2`

#### Scenario: Unknown allocation is rejected

- **WHEN** `allocation` is set to a value other than `port` or `merging`
- **THEN** the configuration fails to load

### Requirement: Transmit address is one scoped multicast base

Under allocation `port`, the transmit multicast address SHALL come from
the daemon multicast settings. The daemon SHALL transmit only when that
address is inside `239.0.0.0/8` and is not `239.255.255.255`. Under
allocation `port`, the daemon SHALL NOT assign a different address to
each source by incrementing the base. Under allocation `merging`, the
group for an unpinned source SHALL follow the merging allocation
requirement instead of a shared base address.

#### Scenario: The SAP group stays down

- **WHEN** allocation is `port` and the multicast base address is
  `239.255.255.255`
- **THEN** no source transmits

#### Scenario: Sources share the base address

- **WHEN** allocation is `port` (or omitted)
- **AND** two sources are transmitting and neither sets its own
  address
- **THEN** both use the configured base multicast address

### Requirement: Transmit ports step from an even base

Under allocation `port`, the daemon SHALL assign each source an even
RTP port of `base_port + 2 * index`, where `index` is that source's
position among configured sources starting at zero. Under allocation
`merging`, every unpinned source SHALL use `base_port` (no port step).
`base_port` SHALL be even. The daemon SHALL NOT send that source's RTP
to the following odd port.

#### Scenario: The first two sources use consecutive even ports

- **WHEN** allocation is `port` (or omitted), `base_port` is 5004, and
  the first two sources are transmitting
- **THEN** their RTP ports are 5004 and 5006

#### Scenario: An odd base port stays down

- **WHEN** `base_port` is odd
- **THEN** no source transmits

### Requirement: Merging allocation derives the group from PTP unicast

When allocation is `merging` and a source does not pin an address, the
daemon SHALL set that source's multicast group to
`239.{session}.{o3}.{o4}`, where `{o3}` and `{o4}` are the third and
fourth octets of the PTP interface IPv4 address, and `{session}` is
`session_base + index` with `index` the source's zero-based position
among configured sources. `session_base` SHALL default to `1`. The
daemon SHALL use the configured even `base_port` for every such source
that does not pin a port. The daemon SHALL NOT require
`[multicast].address` in this mode.

#### Scenario: Two sources follow the host unicast

- **WHEN** allocation is `merging`, `session_base` is `1`, and
  `base_port` is `5004`
- **AND** the PTP interface address is `192.168.60.50`
- **AND** the first two sources have no pinned address or port
- **THEN** their destinations are `239.1.60.50:5004` and
  `239.2.60.50:5004`

#### Scenario: Missing PTP IPv4 holds unpinned merging sources

- **WHEN** allocation is `merging`
- **AND** the PTP interface has no IPv4 address
- **AND** a source does not pin an address
- **THEN** that source does not transmit

#### Scenario: Session octet overflow holds the source

- **WHEN** allocation is `merging`
- **AND** `session_base + index` does not fit in one octet
- **THEN** that source does not transmit

#### Scenario: Merging sources share the base port

- **WHEN** allocation is `merging`, `base_port` is 5004, and the first
  two sources transmit without pinned ports
- **THEN** both use RTP port 5004

#### Scenario: A pin overrides merging allocation

- **WHEN** allocation is `merging` and the PTP interface address is
  `192.168.60.50`
- **AND** the first source pins `address = "239.9.60.50"` and omits
  `port`
- **THEN** that source transmits to `239.9.60.50` at `base_port`

### Requirement: Merging mode rejects a configured base address

When allocation is `merging`, the daemon SHALL reject a configuration
that sets `[multicast].address`. Per-source pinned `address` values
remain allowed.

#### Scenario: Base address with merging fails to load

- **WHEN** `allocation` is `merging` and `[multicast].address` is set
- **THEN** the configuration fails to load

### Requirement: A source may pin address or port

A source MAY set its own multicast address or port under either
allocation mode. A pinned port SHALL be even. Two sources SHALL NOT
transmit to the same address and port.

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
`ptp4l` reports a lock and a grandmaster identity, the absolute PTP
offset is within the configured offset threshold (when that threshold
is non-zero), and the lock has remained continuously true for the
configured lock-hold duration. Losing the lock SHALL remove those nodes
and SHALL reset the hold timer. A later continuous lock for the full
hold duration with offset within threshold SHALL create them again when
they still qualify. The daemon SHALL NOT step or slew any clock.

#### Scenario: Losing the lock removes the stream nodes

- **WHEN** the PTP lock is lost while sources or sinks are active
- **THEN** those RTP nodes are removed

#### Scenario: Sources stay down during the lock hold

- **WHEN** `ptp4l` has just become locked and the configured lock-hold
  duration has not yet elapsed without an unlock
- **THEN** configured sources do not transmit
- **AND** configured sinks do not join a group

#### Scenario: Sources come up after a continuous hold

- **WHEN** `ptp4l` stays locked for the configured lock-hold duration,
  the absolute PTP offset is within the configured threshold (or the
  threshold is zero), and a source is otherwise legal
- **THEN** that source transmits

#### Scenario: Sources stay down while offset is high

- **WHEN** `ptp4l` is locked with a grandmaster, the lock-hold has
  elapsed, the offset threshold is non-zero, and the absolute PTP
  offset exceeds that threshold
- **THEN** configured sources do not transmit
- **AND** configured sinks do not join a group

#### Scenario: Sources come up after hold and offset settle

- **WHEN** `ptp4l` stays locked for the configured lock-hold duration,
  the absolute PTP offset is within the configured threshold, and a
  source is otherwise legal
- **THEN** that source transmits

#### Scenario: Unlock during the hold restarts the timer

- **WHEN** the lock is lost before the hold duration elapses, then
  lock returns
- **THEN** the hold timer starts again from zero
- **AND** sources do not transmit until the full hold elapses again

### Requirement: Lock-hold duration is configurable

The daemon configuration SHALL accept a non-negative lock-hold duration
in milliseconds. When the key is absent, the daemon SHALL use a
non-zero default. A value of zero SHALL bring media nodes up as soon as
`ptp4l` reports locked with a grandmaster (no extra dwell).

#### Scenario: Zero hold matches immediate bring-up

- **WHEN** the lock-hold duration is 0 ms, the clock is locked with a
  grandmaster, and a source is otherwise legal
- **THEN** that source transmits without waiting for a dwell

#### Scenario: Absent key uses the default hold

- **WHEN** the configuration omits the lock-hold key
- **THEN** the daemon applies its default non-zero hold before sources
  may transmit

### Requirement: Offset threshold is configurable

The daemon configuration SHALL accept a non-negative maximum absolute
PTP offset in nanoseconds. When the key is absent, the daemon SHALL
use a non-zero default. A value of zero SHALL disable the offset gate
so media nodes may come up once lock and lock-hold (and other existing
checks) pass.

#### Scenario: Zero threshold disables offset gating

- **WHEN** the offset threshold is 0 ns, the clock is locked with a
  grandmaster, the lock-hold has elapsed, and a source is otherwise
  legal
- **THEN** that source transmits regardless of reported PTP offset

#### Scenario: Absent key uses the default threshold

- **WHEN** the configuration omits the offset-threshold key
- **THEN** the daemon applies its default non-zero threshold before
  sources may transmit

### Requirement: Reads PTP state without spawning a process

The daemon SHALL read `ptp4l` lock state, domain, grandmaster clock
identity, and PTP offset over the read-only management Unix socket in
its own process. It SHALL NOT spawn `pmc` or any other helper for that
read. A missing socket SHALL be treated as unlocked. A failed offset
read while otherwise locked SHALL be treated as offset outside the
threshold when the threshold is non-zero.

#### Scenario: Lock is read with no child process

- **WHEN** the daemon queries the PTP clock
- **THEN** it uses the read-only management socket itself
- **AND** it does not create a child process for that query

#### Scenario: Offset is read with no child process

- **WHEN** the daemon queries the PTP offset
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
active and why it is held down. When the clock is locked but the
lock-hold has not elapsed, the query SHALL report that hold distinctly
from an unlocked clock. When the clock is locked and the hold has
elapsed but the absolute PTP offset exceeds the configured threshold,
the query SHALL report that offset hold distinctly from unlock and
from lock-hold.

#### Scenario: Status reports an unlocked source

- **WHEN** the PTP clock is unlocked and a source is configured
- **THEN** the query reports the clock unlocked and that source held
  down

#### Scenario: Status reports lock hold

- **WHEN** the PTP clock is locked, the lock-hold has not elapsed, and
  a source is configured
- **THEN** the query reports the clock locked and that source held
  down for the lock hold

#### Scenario: Status reports offset hold

- **WHEN** the PTP clock is locked, the lock-hold has elapsed, the
  absolute PTP offset exceeds the configured threshold, and a source
  is configured
- **THEN** the query reports the clock locked and that source held
  down for the offset hold

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

### Requirement: The daemon discovers peer session SDP

The daemon SHALL browse RAVENNA device and session advertisements on the
`ptp4l` interface and SHALL retrieve session SDP over RTSP DESCRIBE for
configured sinks that are not yet established. It SHALL key each result
by the browsed device instance name and session instance name. A
configured sink whose `device` and `stream` match a discovered pair
SHALL use that SDP for qualify (subject to existing multicast, rate,
domain, and PTP gates). DESCRIBE paths SHALL use the session TXT `path`
when present, otherwise `/by-name/<session>`, with path segments
percent-encoded on the request line.

#### Scenario: A matching sink leaves device-absent

- **WHEN** a peer advertises device `Anubis_610267` and session
  `Anubis_610267_Combo 1/2` with acceptable SDP, PTP gates pass, and a
  sink is configured with that device and stream
- **THEN** that sink is not held down for `device-absent`

#### Scenario: Unknown device stays device-absent

- **WHEN** a sink names a device and stream that are not present in the
  discovery results
- **THEN** that sink remains held down for `device-absent`

#### Scenario: Session names with spaces DESCRIBE successfully

- **WHEN** a discovered session instance name contains spaces or
  parentheses and has no TXT `path`
- **THEN** the daemon DESCRIBEs `/by-name/<that name>` with a
  percent-encoded request-URI
- **AND** a successful response is available for sink qualify

### Requirement: Established sinks keep cached SDP

While a sink remains up, the daemon SHALL continue to use the SDP
already obtained for that `(device, stream)` and SHALL NOT re-DESCRIBE
or otherwise re-fetch that session's SDP on each control-loop tick or
discovery refresh. The daemon MAY DESCRIBE again after the sink goes
down, is removed from configuration, or the cached entry is cleared
because the peer advertisement disappeared.

#### Scenario: No DESCRIBE while the sink stays up

- **WHEN** a sink has qualified and is up from a prior DESCRIBE
- **THEN** subsequent control-loop ticks do not DESCRIBE that session
  again while the sink remains up

#### Scenario: DESCRIBE may run again after the sink drops

- **WHEN** a previously up sink becomes held down or is removed from
  configuration, and later a matching peer session is needed again
- **THEN** the daemon may DESCRIBE that session to re-establish SDP
