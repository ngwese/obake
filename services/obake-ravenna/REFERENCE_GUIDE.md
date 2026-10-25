<!--
SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
SPDX-License-Identifier: MIT
-->

# obake-ravenna reference

How the session daemon turns `config.toml` into SDP, mDNS, a PTP graph
clock, and PipeWire RTP nodes. The live file on a host is
`/home/obake/.config/obake-ravenna/config.toml`. A copy with no streams
is seeded from `/usr/share/obake/obake-ravenna/` the first time that
directory is missing. `systemctl --user reload obake-ravenna` re-reads
the file. `obake-ravenna status` prints why each stream is up or held
down.

The sample rate, packet size, multicast address, and channel count all
come from this file. The PTP interface, domain, and grandmaster identity
come from `ptp4l` (`/home/obake/.config/linuxptp/ptp4l.cfg` and its
management socket), not from the TOML file.

## 1. Config file

```toml
[graph]
rate = 48000
node_group = "obake.ravenna"

[ptp]
lock_hold_ms = 2000

[packet]
frames = 48

[multicast]
address = "239.1.60.50"
base_port = 5004
ttl = 15

[[source]]
name = "obake-out-1"
channels = 8
format = "L24"
# payload = 98
# address = "239.1.60.51"
# port = 5010

[[sink]]
name = "obake-in-1"
device = "Anubis"
stream = "Anubis_Combo 1/2"
```

`[multicast]` is required when any `[[source]]` exists. An empty source
and sink list loads the daemon, publishes nothing, and creates no RTP
nodes. The image seed is that empty file.

### `[graph]`

| Key | Default | Effect |
| --- | --- | --- |
| `rate` | `48000` | Media-clock rate in Hz for every stream. SDP `a=rtpmap` uses it. PipeWire `audio.rate` uses it. A remote sink whose SDP rate differs is held down (`rate-mismatch`). |
| `node_group` | `obake.ravenna` | PipeWire `node.group` shared by the PTP driver and the RTP nodes. The driver with this group owns their clock. Leave application nodes out of this group. |

Legal rates: 44100, 48000, 88200, 96000, 176400, 192000, 352800, 384000.
Anything else holds every stream down (`bad-sample-rate`).

### `[ptp]`

| Key | Default | Effect |
| --- | --- | --- |
| `lock_hold_ms` | `2000` | After `ptp4l` becomes FOLLOWER, sources and sinks stay held down until that lock has remained continuously true for this many milliseconds. Any unlock resets the timer. `0` brings media nodes up on the lock edge (previous behavior). |

Status still reports `ptp: locked` / `ptp: unlocked` from `ptp4l`. While locked but waiting on this dwell, streams show `held-down (lock-hold)`.

### `[packet]`

| Key | Default | Effect |
| --- | --- | --- |
| `frames` | `48` | Samples in one RTP packet. SDP `a=framecount` and PipeWire `rtp.framecount` both use it. This is the packet size, not the PipeWire quantum. |

At 48000 Hz the legal values are 6, 12, 16, 32, 48, 64, and 192 samples
(packet times 0.125, 0.25, 1/3, 2/3, 1, 4/3, and 4 ms). The same
absolute counts are accepted on the 44.1 kHz family. Other counts hold
transmitters down (`bad-frame-size`).

`sess.latency.msec` is not a config key. The daemon sets it to three
packet times (3 ms when `frames` is 48 at 48000 Hz). That latency is how
much audio the RTP node keeps queued so one graph cycle can emit several
packets. It is not added to the RTP timestamp and it is not written into
the SDP.

### `[multicast]`

| Key | Default | Effect |
| --- | --- | --- |
| `address` | required with sources | IPv4 group in `239.0.0.0/8` shared by sources that do not pin their own address. SDP `c=` and the RTP destination use it. |
| `base_port` | `5004` | Even RTP port for source 0. Source `i` uses `base_port + 2*i` unless that source pins `port`. The odd port (`port+1`) is the RTCP pair in the RTSP `Transport` line. |
| `ttl` | `15` | IPv4 multicast TTL on transmitted packets and on SDP `c=` (`address/ttl`). |

`239.255.255.255` is rejected (SAP announcement group). An odd
`base_port` or odd pinned `port` is rejected. Two sources that resolve
to the same group and port are both held down (`duplicate-destination`).

### `[[source]]` (local audio toward the network)

| Key | Required | Effect |
| --- | --- | --- |
| `name` | yes | PipeWire `node.name`, RTSP path `/by-name/<name>`, SDP session name `s=`, and the mDNS session instance name. |
| `channels` | yes | Channel count in SDP `a=rtpmap` and in the RTP payload. One input port is created per channel. |
| `format` | yes | `L16` (16-bit big-endian, 2 bytes/sample, PipeWire `S16BE`) or `L24` (24-bit packed, 3 bytes/sample, PipeWire `S24BE`). |
| `payload` | no | RTP dynamic payload type, 96 through 127. Default is `96 + channels` (8 channels → 104). Set this when a receiver expects a specific type (Merging devices often use 98). |
| `address` | no | Pin this source to its own multicast group. Otherwise `[multicast].address`. |
| `port` | no | Pin an even RTP port. Otherwise `base_port + 2*index`. |

A source stays down until `ptp4l` is FOLLOWER, the lock-hold dwell has
elapsed, a non-zero grandmaster identity is known, the PTP interface
exists and has an IPv4 address, and the packet fits in the payload cap
below. `obake-ravenna status` prints the reason: `unlocked`,
`lock-hold`, `no-grandmaster`, `interface-missing`, `bad-destination`,
`bad-payload-type`, `payload-too-large`, and the others in the README
table.

### `[[sink]]` (network audio toward a local node)

| Key | Required | Effect |
| --- | --- | --- |
| `name` | yes | Local PipeWire node name for the receiver. |
| `device` | yes | Remote mDNS instance name (the other host's node record, usually its hostname). |
| `stream` | yes | Remote session name. The daemon looks up SDP for the pair `(device, stream)`. |

Qualify will bring a sink up only when that pair has been discovered,
the remote `c=` address is in `239.0.0.0/8`, the remote rate equals
`[graph].rate`, and the remote PTP domain equals local `ptp4l`. Otherwise
the sink is `device-absent`, `unicast-sdp`, `rate-mismatch`, or
`domain-mismatch`.

The control loop browses `_ravenna._sub._rtsp._tcp` and then passes an
empty discovery map into qualify. A configured `[[sink]]` therefore stays
`device-absent` until peer DESCRIBE results are fed into that map. The
keys above are the ones that qualify and the RTP source module will use
once discovery is connected. Transmitters do not depend on it.

### Payload size and MTU

One packet's audio payload is:

```text
channels * frames * bytes_per_sample
```

`L16` is 2 bytes, `L24` is 3. The daemon allows at most 1440 bytes of
audio. That cap sits under a 1500-byte Ethernet MTU after IPv4 (20), UDP
(8), and RTP (12). The module is always given `net.mtu = 1500`. There is
no config key to raise it, and the daemon does not fragment.

| Channels | Format | Frames | Payload | Result |
| --- | --- | --- | --- | --- |
| 8 | L24 | 48 | 1152 | transmitted |
| 8 | L24 | 64 | 1536 | `payload-too-large`, no RTP node |
| 8 | L16 | 64 | 1024 | transmitted |
| 2 | L24 | 48 | 288 | transmitted |

When a combination exceeds 1440 bytes the choices are: fewer channels,
a smaller legal `frames` value, or `L16` instead of `L24`. Latency does
not enter this check. `sess.latency` only changes how many of those
packets the graph emits per cycle.

Linking fewer channels than `channels` does not shrink the SDP. PipeWire
can renegotiate the node format down to the linked port count, so the
packets get shorter while `a=rtpmap` still announces the configured
count. A receiver that checks payload length against the SDP will
reject that stream. Connect every advertised channel, or set `channels`
to the number you actually link.

## 2. SDP

Transmitters serve SDP on RTSP `DESCRIBE /by-name/<source name>` at
port 8554. The body is built only for a source that qualified. A held-down
source answers 404. The builder always emits this shape (values from an
8-channel L24 source at 48 kHz, 48-sample packets, payload 98, TTL 15):

```text
v=0
o=- 0 0 IN IP4 192.168.60.50
s=obake-out-1
c=IN IP4 239.1.60.50/15
t=0 0
a=clock-domain:PTPv2 0
a=ts-refclk:ptp=IEEE1588-2008:30-D6-59-FF-FE-01-D9-43:0
a=mediaclk:direct=0
m=audio 5004 RTP/AVP 98
c=IN IP4 239.1.60.50/15
a=rtpmap:98 L24/48000/8
a=source-filter: incl IN IP4 239.1.60.50 192.168.60.50
a=clock-domain:PTPv2 0
a=sync-time:0
a=framecount:48
a=palign:0
a=ptime:1
a=ts-refclk:ptp=IEEE1588-2008:30-D6-59-FF-FE-01-D9-43:0
a=mediaclk:direct=0
a=recvonly
```

Session-level lines and media-level lines repeat the clock attributes
on purpose. Merging devices read the media-level copies.

| Line | Plain meaning | What drives it |
| --- | --- | --- |
| `v=0` | SDP version. Always 0. | Fixed. |
| `o=- 0 0 IN IP4 <ip>` | Origin. The username is `-`, the session id and version are 0, and the address is the unicast IPv4 of the PTP interface. Peers use that address as the sender. The id `0` is a legal opaque counter; other hosts use other numbers. | Interface address from `ip addr` on the `ptp4l` NIC. Not a config key. |
| `s=<name>` | Session name peers display. | `[[source]].name`. |
| `c=IN IP4 <group>/<ttl>` | Where to listen, and the multicast scope. The `/ttl` suffix is the hop limit. | `[multicast].address` or the source's pinned `address`, and `[multicast].ttl` (default 15). |
| `t=0 0` | The session is permanent (no start or stop time). | Fixed. |
| `a=clock-domain:PTPv2 <domain>` | PTP domain the media clock follows. Anubis shows this as CLOCK DOMAIN. | `ptp4l` domain, not the TOML file. |
| `a=ts-refclk:ptp=IEEE1588-2008:<gmid>:<domain>` | The grandmaster identity and domain. Receivers lock the stream to this clock. | `ptp4l` parent data set (8-byte clock id, printed as hex bytes) and domain. |
| `a=mediaclk:direct=0` | The RTP timestamp *is* the media-clock sample count since the PTP epoch. The offset is zero, so a legal timestamp is a multiple of `framecount`. | Fixed at 0. See section 5 for why the sender has to quantize to make this true. |
| `m=audio <port> RTP/AVP <pt>` | UDP port and payload type. `RTP/AVP` is RTP audio/video profile. | Port from `[multicast].base_port` plus stepping, or the source's pinned `port`. Payload type from `payload` or `96 + channels`. |
| `c=IN IP4 <group>/<ttl>` | Same destination, repeated at media level so a receiver that only reads the media block still joins the right group. | Same as the session `c=` line. |
| `a=rtpmap:<pt> L24/48000/8` | Encoding, rate, and channel count for that payload type. `L24` is packed 24-bit big-endian; `L16` is 16-bit big-endian. | `format`, `[graph].rate`, `channels`, and the payload type. |
| `a=source-filter: incl IN IP4 <group> <ip>` | Source-specific multicast: accept this group only from the sender's unicast address. | Group plus the PTP interface address. |
| `a=sync-time:0` | No extra RAVENNA sync offset on top of the media clock. | Fixed. |
| `a=framecount:48` | Samples per packet. Together with `mediaclk:direct=0` this is the grid the timestamp must land on (`timestamp % 48 == 0`). | `[packet].frames`. |
| `a=palign:0` | Packet alignment phase is 0, the same grid as `framecount`. | Fixed. |
| `a=ptime:1` | Packet duration in milliseconds (`frames / rate * 1000`). | Derived from `[packet].frames` and `[graph].rate`. |
| `a=recvonly` | Merging's session record is written from the receiver's point of view: the peer receives this multicast. The daemon is still the sender. | Fixed. |

`SETUP` answers with
`Transport: RTP/AVP;multicast;destination=<group>;port=<port>-<port+1>;ttl=<ttl>`
and a session id. `PLAY` and `TEARDOWN` use that id. The multicast
stream itself starts when the source qualifies; it does not wait for
`PLAY`.

## 3. mDNS records

The daemon talks to Avahi on the system bus. It publishes IPv4 only
(`AVAHI_PROTO_INET`) on the `ptp4l` interface index. An IPv6 SRV that
never resolves makes Anubis leave RTSP HOST blank, so the record is not
announced for IPv6.

Both records use DNS-SD type `_rtsp._tcp` and port **8554**. A subtype
tells RAVENNA browsers which list the record belongs in.

### Node record

| Field | Value |
| --- | --- |
| Instance name | Contents of `/etc/hostname`, or `obake` when that file is empty. |
| Type | `_rtsp._tcp` |
| Subtype | `_ravenna._sub._rtsp._tcp` |
| Port | 8554 |
| TXT | Empty. |

Device lists browse the subtype. This record means "a RAVENNA node is
here, and its RTSP port is 8554." It is published once the PTP interface
has an ifindex, including when no session is up. It is withdrawn when
that interface disappears.

### Session record (one per transmitting source that is up)

| Field | Value |
| --- | --- |
| Instance name | `[[source]].name` |
| Type | `_rtsp._tcp` |
| Subtype | `_ravenna_session._sub._rtsp._tcp` |
| Port | 8554 |
| TXT | `path=/by-name/<name>` |

Session browsers use the subtype to find streams rather than devices.
The TXT path is the RTSP DESCRIBE path. Anubis uses it to fill RTSP HOST
and then fetch the SDP in section 2. A source that is held down is not
published. Removing it from the config, or losing PTP lock, withdraws
the record.

The daemon also browses `_ravenna._sub._rtsp._tcp` so it can see peer
nodes. Those browse results are not yet turned into SDP for `[[sink]]`.

## 4. Session-level PipeWire objects

The daemon process runs a PipeWire main loop on a thread named
`obake-ravenna-pw`. That thread owns two different connections:

| Connection | `application.name` | Where the objects live | Role |
| --- | --- | --- | --- |
| Context | `obake-ravenna [rtp]` | Inside the daemon process | `pw_context_load_module` loads `libpipewire-module-rtp-sink` and `libpipewire-module-rtp-source`. Each module connects to the user daemon and exports its node. The module's main loop has to keep running or the node disappears. |
| Core | `obake-ravenna [session]` | Client of the user `pipewire` service (`/run/user/<uid>/pipewire-0`) | Creates and destroys the PTP driver *inside* the PipeWire daemon, where it can be the graph clock. |

### PTP driver

Created with `spa-node-factory` on the session core whenever the `ptp4l`
interface exists. Removed when that interface is gone. There is one
driver, named `obake-ptp-driver`, shared by every RTP node.

```text
factory.name    = support.node.driver
node.name       = obake-ptp-driver
node.group      = <graph.node_group>
priority.driver = 100000
clock.interface = <ptp4l interface>
resync.ms       = 5.0
```

| Property | Why it is set |
| --- | --- |
| `support.node.driver` | A clock node with no audio ports. It wakes the graph. |
| `node.name` | Fixed. `pw-cli` shows `obake-ptp-driver`. |
| `node.group` | `[graph].node_group`. Every RTP node in this group takes its clock from the highest-priority driver in the group. |
| `priority.driver` | 100000, high enough that this driver wins the group over WirePlumber's dummy driver. |
| `clock.interface` | The NIC named in `ptp4l.cfg`. The driver reads that NIC's PTP hardware clock directly. It does not read `CLOCK_REALTIME`, so `phc2sys` is not part of this path. |
| `resync.ms` | How far the graph position may drift from the hardware clock before the driver snaps the position. Fixed at 5.0 in the daemon. |

`clock.interface` advances a sample counter. On start or resync it snaps
that counter to the hardware clock, then adds one quantum per cycle and
only nudges the wake-up time to stay near the clock. The snap is not
rounded to `[packet].frames`. Whatever remainder the snap lands on stays
for the life of the stream, because each later cycle adds a whole number
of samples.

A `resync.ms` of 1.5 is 72 samples at 48 kHz. The position sits about 74
samples off the hardware clock (a fixed phase, about 1.5 ms, not a
walking rate error). That tripped a reset on every cycle, the timestamp
phase jumped, and receivers muted. 5.0 ms is wide enough to absorb that
phase, so the position stays put and the remainder stays constant.

The driver is not an RTP object. It has no ports to link. Application
nodes (a tone, a ChucK instance, a hardware ALSA device) stay out of
`node.group`. Putting them in the group asks them to run on the PTP
wake-up; a JACK client or `audiotestsrc` then fails to start or fights
the driver. They follow the graph through ordinary links instead.

WirePlumber is left in place. The daemon does not retune the global
graph rate and does not install a `pipewire.conf.d` fragment. RTP nodes
are created and destroyed by the client above as streams qualify.

## 5. RTP objects, context, and clocking

RTP modules are loaded in the daemon process's own context
(`obake-ravenna [rtp]`), not inside `pipewire.service`. From there each
module connects to the user daemon and exports one node. The node name
is `[[source]].name` or `[[sink]].name`. `node.autoconnect` is false, so
nothing is linked until you link it.

A source that qualifies loads `libpipewire-module-rtp-sink` (local audio
in, multicast RTP out). A sink that qualifies loads
`libpipewire-module-rtp-source` (multicast RTP in, local audio out).
Replacing a stream's arguments unloads and reloads that module. A reload
picks a new graph-position phase.

### Transmitter (`rtp-sink`) line by line

`from_source` builds the module arguments. Indented keys under
`stream.props` are the exported node's properties. The rest are module
arguments. The block below is what an 8-channel L24 source produces
when `frames` is 48.

```text
local.ifname       = "<ptp4l NIC>"          # egress device for the multicast
destination.ip     = "<group>"              # [multicast].address or source pin
destination.port   = <even port>            # base_port + 2*index, or source pin
net.mtu            = 1500                   # fixed; payload cap is separate
net.ttl            = <ttl>                  # [multicast].ttl, default 15
net.loop           = false                  # do not loop our packets back locally
sess.min-ptime     = <ptime>                # packet time, locked
sess.max-ptime     = <ptime>                # same, so the packet size cannot float
sess.latency.msec  = <3 * ptime>            # queue depth, not a timestamp offset
sess.name          = "<source name>"
sess.media         = "audio"
sess.ts-refclk     = "ptp=IEEE1588-2008:<gmid>:<domain>"
sess.ts-offset     = 0                      # no extra constant
sess.ts-direct     = true                   # stamp the graph position
sess.ts-quantize   = true                   # then snap onto framecount
audio.format       = "S24BE"                # or S16BE
audio.rate         = <graph.rate>
audio.channels     = <channels>
stream.props = {
    node.name           = "<source name>"
    media.class         = "Audio/Sink"      # other nodes connect *into* it
    node.virtual        = true
    device.api          = "aes67"
    node.always-process = true              # keep emitting with silence on open ports
    node.group          = "<graph.node_group>"
    node.autoconnect    = false
    rtp.framecount      = <packet.frames>
    rtp.payload         = <payload type>
    rtp.ntp             = 0                 # no NTP clock mapping
    rtp.fetch-ts-refclk = true
}
```

`rtp.framecount` has to be inside `stream.props`. The stock module
copies only a fixed set of top-level keys onto the stream.

`media.class = Audio/Sink` means the node consumes audio. Its input
ports are named `send_1`, `send_2`, … when no channel map is set. Link
a producer into those ports. The ports show up in `pw-link -i` once the
source is up.

### Why `sess.ts-quantize` exists

`a=mediaclk:direct=0` tells the receiver that RTP timestamp 0 is the PTP
epoch and that every packet starts on a `framecount` boundary. Anubis
checks that (`timestamp % framecount == 0`). A non-zero remainder is
"Wrong RTP SAC": the stream is marked receiving and then muted, and the
ahead-time trace sits on a flat 12000 µs instead of the real delay.

`sess.ts-direct` makes `rtp-sink` write the graph clock position into
the packet, then add `framecount` for each following packet in the
cycle. The driver's position is snapped to the hardware clock once and
then stepped by the quantum. That snap is not a multiple of
`framecount`, so every packet in the stream keeps the same remainder.
Restarting the module picks a different remainder. `sess.ts-offset`
cannot correct it: the offset is applied at load, and the next load
draws a new phase, so the correction never settles.

Stock PipeWire 1.6.9 also drops `sess.ts-direct` before it reaches the
stream. The stamp is then `position_at_sync + current_position`, which
is off by a huge constant as well as by the remainder. The image patch
`image/os/buildroot/package/obake-pipewire/0001-rtp-sink-quantize-timestamps.patch`
forwards `sess.ts-direct` and adds `sess.ts-quantize`. When that flag is
set, the value written into the RTP header is rounded down:

```text
timestamp -= timestamp % framecount
```

Full packets already step by `framecount`, so they stay one packet
apart. The flag defaults off in the module. The daemon sets it only on
transmitters. Without the patched module the property is ignored and the
SAC check fails again.

Rounding down spends up to one packet of lead (just under 1 ms at 48
samples / 48 kHz). Combined with a driver that runs about 1.5 ms ahead
of the hardware clock, Anubis typically measures about 1000–1100 µs of
ahead time. Peers that timestamp further ahead sit at 2–4 ms. The trace
staying above zero is what keeps the stream unmuted. `sess.latency`
does not move that number; it only queues audio inside the sender.

`aes67.driver-group` (a separate PTP sender node inside `rtp-sink`) can
also emit aligned timestamps, and then stops sending: the sender waits
to refill while the audio node is clocked by something else. The daemon
does not set that option. Clocking is `node.group` plus the driver
above.

### Receiver (`rtp-source`)

The same module argument block is used, with these differences:

| Argument | Receiver value |
| --- | --- |
| Module | `libpipewire-module-rtp-source` |
| Address keys | `source.ip` and `source.port` (the group and port from the remote SDP) |
| `net.ttl` | Omitted. |
| `sess.ts-refclk` | `ptp=traceable` |
| `sess.ts-quantize` | Omitted. Quantize applies to outgoing stamps. |
| `media.class` | `Audio/Source` (other nodes connect *from* it) |
| `audio.*`, `rtp.framecount`, `rtp.payload` | Taken from the remote SDP, not from `[[sink]]`. |

`[[sink]].name` is only the local node name. Channels, format, rate, and
packet size come from the peer. The rate must already equal
`[graph].rate` or the sink stays down.

## 6. Connecting native nodes and JACK shapes

RTP nodes do not autoconnect. After `obake-ravenna status` shows a
source or sink up, link them by hand. Links are not saved across a
daemon restart.

List ports:

```sh
pw-link -o    # producers, including obake-in-* once a sink is up
pw-link -i    # consumers, including obake-out-*:send_1 …
```

### A PipeWire node into a transmitter

The transmitter is an `Audio/Sink`. Your node's output ports connect to
its `send_N` ports. The producer stays out of `[graph].node_group` and
runs at `[graph].rate`.

```sh
pw-link my-source:output_FL obake-out-1:send_1
pw-link my-source:output_FR obake-out-1:send_2
```

Port names on the producer depend on that node. Names on `obake-out-1`
are `send_1` … `send_<channels>` when the daemon did not set a channel
map. Link all `channels` ports. A 2-channel tone linked into an
8-channel source renegotiates the node to 2 channels and emits short
packets while the SDP still says 8.

A node you only want as a clock follower of the hardware clock, with no
audio of its own, does not belong in this group. The PTP driver already
clocks the RTP nodes. Feed them with normal links.

### A PipeWire node from a receiver

The receiver is an `Audio/Source`. Its output ports connect to your
node's inputs:

```sh
pw-link obake-in-1:capture_1 my-sink:input_FL
pw-link obake-in-1:capture_2 my-sink:input_FR
```

Confirm the receiver's port names with `pw-link -o` after the sink
qualifies. They follow the remote channel count. Your sink stays out of
`node.group` the same way.

Remove a link with `pw-link -d <out> <in>`.

### JACK shapes (ChucK and the same pattern)

Host PipeWire is the JACK server. Shapes do not start `jackd`. The
`chuck` image runs `pw-jack /usr/local/bin/chuck`, which loads PipeWire's
`libjack`. Build and runtime details, including the Trixie client against
the host's PipeWire 1.6.9, are in `shapes/docs/pipewire-jack.md`.

Run the container as `obake` with the user runtime directory visible, or
the client cannot open the host socket:

```sh
runtime="/run/user/$(id -u)"
singularity run \
  --bind "$runtime" \
  --env "XDG_RUNTIME_DIR=$runtime" \
  chuck.sif my-patch.ck
```

ChucK's JACK ports then appear in `pw-link`. Connect them to the RTP
nodes the same way as any other client. Leave the ChucK node out of
`obake.ravenna`:

```sh
pw-link -iol
pw-link 'chuck:outport 1' obake-out-1:send_1
pw-link 'chuck:outport 2' obake-out-1:send_2
pw-link obake-in-1:capture_1 'chuck:inport 1'
```

The quoted names are illustrative. Use the names `pw-link` prints.
`rnbo-runner` is the same kind of `pw-jack` client and uses the same
links. A shape that only opens ALSA sequencer MIDI does not need
`pw-jack` and does not touch these nodes.

WirePlumber will not link these for you while `node.autoconnect` is
false. A small script that waits for the RTP node and calls `pw-link` is
the way to restore connections after `obake-ravenna` restarts.
