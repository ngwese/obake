# Design

## Context

See proposal.md for why this exists. Constraints that shape the
approach:

- `host-audio` already runs one PipeWire 1.6.9 daemon and WirePlumber
  0.5.18 as `obake` user services, at 48000 Hz and quantum 128.
  Factory config has no AES67 or RAVENNA ALSA profile. Linger starts
  that user manager at boot.
- `ptp4l` on `qdl` is a system unit running as uid `obake`, with
  ambient `CAP_NET_ADMIN`, `CAP_NET_BIND_SERVICE`, and `CAP_SYS_TIME`.
  It is disabled until the operator enables it. Its config is
  `/home/obake/.config/linuxptp/ptp4l.cfg`. The user manager cannot
  grant those capabilities. `phc2sys` is a separate unit.
- The `ptp4l` unit has no `RuntimeDirectory=`. The seeded config does
  not set `uds_address`, so the default `/var/run/ptp4l` is not a path
  uid `obake` can create.
- Avahi already owns UDP 5353 on every board. `obake-runtime` selects
  it for the shared base. `BR2_PACKAGE_LINUXPTP` is selected only in
  `configs/obake-qdl.config`.
- The image build is Buildroot 2026.02.3. It cross-compiles and does
  not run target binaries. There is no Rust package in the external
  tree today. `builders/builder-base` has a host Rust toolchain for
  developer builds.
- `/home` survives OTA. `/usr` is the read-only slot.

## Goals / Non-Goals

**Goals:**

- Put the control plane (DNS-SD, RTSP, SDP, lock polling, node
  lifecycle) in one `obake` user service, and leave RTP in PipeWire's
  existing modules.
- Keep WirePlumber the ALSA session manager and leave the graph rate
  and quantum alone.
- Make stream up/down decisions testable without a NIC or a peer.

**Non-Goals:**

- SAP announcement, RTCP, unicast, or resampling.
- Retuning the graph, opening ALSA devices, or slewing any clock.
- A shape, a system unit, or a `Requires=` edge from the user manager
  onto `ptp4l`.
- Enabling `ptp4l` or `phc2sys`, or changing the AES67 client profile.
- Installing the daemon on boards other than `qdl`.

## Decisions

### 1. Cargo tree under services/, packaged only for qdl

The Rust sources live in `services/obake-ravenna/`, beside
`shapes/`, `image/`, and `builders/`. A Buildroot package
`image/os/buildroot/package/obake-ravenna/` builds that tree
with the pinned Buildroot cargo infrastructure and links it against
`obake-pipewire`'s staging `libpipewire`. `configs/obake-qdl.config`
selects the package. `obake_base.config` and `obake-runtime` do not, so
other boards do not download a host Rust toolchain and do not install
the binary.

The package installs:

- `/usr/bin/obake-ravenna`
- the user unit and a `default.target.wants` symlink
- `/usr/share/obake/obake-ravenna/config.toml`, the empty seed
- a tmpfiles snippet of type `C` for the home path, same pattern as
  `etc/tmpfiles.d/obake-linuxptp.conf`
- a system-bus policy that lets uid `obake` browse and publish on
  Avahi

Alternative considered: a shape started by `obake-launcher`. Rejected.
The launcher owns portable workloads under `/persist/shapes`. This
daemon has to be up with PipeWire before any shape is configured.
Alternative considered: selecting it from `obake-runtime`. Rejected.
That meta-package is on every board, and the other boards have no
`ptp4l`.

### 2. User unit, lock checked in the process

`obake-ravenna.service` is a user unit with `After=pipewire.service`
and `WantedBy=default.target`. It runs the same binary as
`obake-ravenna run`. It does not order against the system
`ptp4l.service`. A user unit cannot `Requires=` a system unit, and
`ptp4l` is supposed to stay disabled until the operator enables it.
Inactive or unlocked `ptp4l` is just the hold-down state the spec
already requires.

The process polls `ptp4l` in-process on the read-only management
Unix socket (`uds_ro_address`). It sends IEEE 1588 management
messages for `GET PORT_DATA_SET` and `GET DOMAIN`, parses the TLV
replies into port state and domain, and keeps the socket open across
the ~2 s tick so each poll is a short UDS round-trip, not a fork.
Spawning `pmc` is rejected: a periodic `exec` would add jitter on the
same host that runs the media graph. Streams are loaded only while
the port state is locked (SLAVE). Unlock unloads the RTP nodes and
leaves the user service running.

The daemon reads the clock. It does not call `clock_adjtime` and it
does not open `/dev/ptp*` for writing. `pmc` remains on the image for
operators; the daemon does not invoke it at runtime.

### 3. Management socket the seed can create

`ptp4l.service` in the `qdl` overlay gains `RuntimeDirectory=ptp4l`,
so systemd creates `/run/ptp4l` owned by `obake` before exec. The
seeded `ptp4l.cfg` sets `uds_address` and `uds_ro_address` under that
directory. The daemon opens the read-only socket. The AES67 keys in
the seed stay as they are, and the preset that leaves the unit
disabled stays as it is.

An existing home `ptp4l.cfg` is not rewritten. The user guide tells an
operator who already has one to add those two keys. Until they do,
the UDS client fails to connect and every stream stays down, which is
the unlocked behavior.

### 4. One config file, re-read on reload

The live file is `/home/obake/.config/obake-ravenna/config.toml`.
The seed is an empty list of sources and sinks, with comments that
name the fields. `ExecReload` sends `SIGHUP`. The process re-reads the
file and reconciles nodes. PipeWire is not restarted.

```toml
[[source]]
name = "obake-out-1"
channels = 8
format = "L24"          # L16 or L24
frame_size = 48         # samples per packet
destination = "239.69.1.10:5004"
ttl = 15                # optional

[[sink]]
name = "obake-in-1"
device = "Studio-IO"    # DNS-SD instance name
stream = "PGM-1"
```

The sample rate is not a field. It is PipeWire's live `clock.rate`,
read when a node is created. The interface is not a field. It is the
non-`global` section header in `ptp4l.cfg`. `REPLACE_WITH_INTERFACE`
is not a present netdev, so streams stay down until the operator
replaces it.

Accepted graph rates are the AES67 / RAVENNA set: 44100, 48000, 88200,
96000, 176400, 192000, 352800, and 384000 Hz. Frame size is legal when
`frame_size / rate` is one of 1/8 ms, 1/4 ms, 1/3 ms, 2/3 ms, 1 ms,
4/3 ms, or 4 ms on 48 kHz-family rates (at 48000 Hz: 6, 12, 16, 32, 48,
64, or 192 samples). On the 44.1 kHz family those ptimes are not an
integer number of samples, so the same absolute sample counts as at
48000 Hz are accepted.
SDP `a=ptime` is that duration. SDP `a=framecount` is the configured
sample count. `a=ts-refclk` carries the local domain from `ptp4l`.

A destination with no port uses 5004. The next odd port is not used.
TTL defaults to 15. Duplicate group-and-port pairs hold both sources
down.

### 5. Control plane here, media plane in PipeWire modules

For each source that qualifies, the daemon loads
`libpipewire-module-rtp-sink` on the existing daemon. For each sink
that qualifies, it loads `libpipewire-module-rtp-source`. Arguments
carry the rate, format (`S16BE` for L16, `S24BE` for L24), channel
count, `sess.latency.msec` derived from the frame size, destination
or source address, port, TTL, and the PTP reference clock. Egress and
the IGMP join use `IP_MULTICAST_IF` with the `ptp4l` interface
address. The daemon does not set `SO_BINDTODEVICE`, which would need
`CAP_NET_RAW`.

The same-user PipeWire socket accepts `load-module` from this client.
A `pipewire.conf.d` fragment is added only if that access check fails,
and it ships in this package so other boards' PipeWire config is
untouched. No ALSA profile is added, and WirePlumber is not given a
script. These nodes are not client streams; WirePlumber's device
policy is not asked to link them.

Node identity is the configured `name`, so a reload can unload the
previous module and load a new one when the SDP or the file changes.

### 6. Avahi for discovery, RTSP for SDP

The daemon uses the system bus to browse and publish
`_ravenna._sub._rtsp._tcp`. It does not open UDP 5353. Publication is
limited to the `ptp4l` interface index.

The process listens on an unprivileged TCP port and answers DESCRIBE
for a source by its configured name. The path used for peers is
`/by-name/<stream>`. That path builder stays isolated so a later
device can add another pattern without a spec change. A source that is
not transmitting has no SDP to give.

A sink DESCRIBEs the browsed device at that path, parses `c=`, `m=`,
`rtpmap`, `ptime` or `framecount`, and `ts-refclk`, and joins only
when the address is multicast, the rate equals the graph, and the PTP
domain matches. A changed SDP unloads the previous node first.

### 7. Status is a subcommand

`obake-ravenna status`, run as `obake`, talks to a socket in the
user runtime directory and prints the lock state plus each configured
stream and its hold-down reason (`unlocked`, `bad-sample-rate`,
`bad-frame-size`, `bad-destination`, `duplicate-destination`,
`interface-missing`, `device-absent`, `unicast-sdp`, `rate-mismatch`,
`domain-mismatch`).
The user unit does not need realtime limits. Packetization runs inside
PipeWire, which already has them.

### 8. Tests that do not need a board

Pure functions cover frame-size legality, destination checks,
duplicate detection, and SDP parse. `cargo test` for those runs on a
developer machine or in `builder-base` and does not link PipeWire.
`image/verification/guest/obake-verify.sh` checks the booted image:
on `qdl`, the binary exists, the user unit is active, the seeded
config defines no streams, and `ptp4l` is still inactive; on every
other board, the binary is absent. A PTP lock and a real RAVENNA peer
stay a board test. `image/docs/user-guide.md` gains a short section
next to the existing PTP section.

## Risks / Trade-offs

- [Host Rust toolchain is large] → Only the `qdl` fragment selects
  the package, so only that build downloads it. Accepted for a
  daemon that must link the target `libpipewire`.
- [PipeWire 1.6.9 RTP module rejects a `sess.ts-refclk` value] → The
  SDP we serve still carries the domain from the management-socket
  reply. The module argument falls back to the module's PTP clock
  token. The spec checks the SDP, not the module property name.
- [Management TLV layout drifts from linuxptp 4.4] → Unit tests pin
  captured binary replies from this suite; `pmc` stays available for
  operators to cross-check.
- [A peer does not use `/by-name/<stream>`] → The path builder is one
  function. The first board test records the path that device uses.
- [Stock Avahi policy already allows publish, and our snippet is
  redundant] → The snippet is an explicit allow for `obake`. It does
  not widen the bus for other users.
- [Operator enables `ptp4l` with the placeholder interface] → No
  netdev matches, streams stay down, status says `interface-missing`.
- [Existing `ptp4l.cfg` has no `uds_address`] → Streams stay down
  until the two keys are added. The seed path is what the spec
  requires. The user guide covers the edit.
- [Reload races a peer's DESCRIBE] → DESCRIBE reads the current
  generation. A source mid-reload is not transmitting, so it has no
  SDP to serve.

## Migration Plan

An image update adds the binary, the enabled user unit, the Avahi
policy, and the empty config seed. First boot copies the seed only
when the home file is missing. No stream starts transmitting because
of the update.

The same update adds `RuntimeDirectory=ptp4l` and the two `uds_*`
keys in the seed template. A home `ptp4l.cfg` that already exists is
left in place. The user guide tells the operator to add
`uds_address` and `uds_ro_address` under `/run/ptp4l` before expecting
lock state. `ptp4l` stays disabled. `phc2sys` is untouched.

Rollback boots the previous slot. That slot has no user unit and no
binary. The home config remains and is unused. A `ptp4l.cfg` the
operator edited with the socket keys is still valid for linuxptp 4.4
on the previous slot; the socket directory exists only while the new
unit's `RuntimeDirectory=` is what starts `ptp4l`. On the old unit the
operator can keep the previous socket path.

## Open Questions

None.
