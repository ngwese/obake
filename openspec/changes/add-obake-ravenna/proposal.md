# Proposal

## Why

A Quieter DL can lock a PTP hardware clock and run a PipeWire graph, and
it still cannot join a RAVENNA network. PipeWire's RTP modules carry the
audio and announce with SAP. RAVENNA peers announce with DNS-SD and
describe streams with RTSP. The host needs a resident session daemon that
connects those, from the operator's configuration, on the NIC that
disciplines the clock.

## What Changes

- The `qdl` image installs `obake-ravenna` as a user service of the
  `obake` account, started at boot with PipeWire. Other boards do not
  install it.
- The daemon browses and publishes RAVENNA RTSP services through the host
  Avahi daemon, and it retrieves and serves stream SDP over RTSP.
- Live configuration is `/home/obake/.config/obake-ravenna/`. The
  `obake` user edits it. A seed creates it only when missing, and the
  seed defines no streams.
- Each configured sink names a remote device and stream, takes the
  multicast address and the media parameters from that stream's SDP, and
  receives multicast only. A sink whose SDP sample rate differs from the
  live PipeWire graph rate stays down.
- Each configured source transmits multicast. Its sample rate is the live
  graph rate. Format, channel count, frame size (samples per packet), and
  multicast destination come from the configuration. The SDP packet time
  is derived from the frame size and the graph rate.
- RTP nodes exist only while `ptp4l` reports a lock, and only on the
  interface named in the `ptp4l` configuration. Losing the lock, or a
  changed SDP, removes those nodes. The graph rate and quantum stay as
  `host-audio` specifies them. WirePlumber keeps owning ALSA devices.
- Lock and domain are read in-process over the read-only `ptp4l`
  management Unix socket. The daemon does not spawn `pmc` or another
  helper for that poll, so the periodic check does not fork.
- `phc2sys` and the shapes are unchanged. The seeded `ptp4l`
  configuration gains a management socket the `obake` user can open, so
  the daemon can read lock state. The PTP profile and the disabled
  boot state of `ptp4l` stay as they are.

## Capabilities

### New Capabilities

- `obake-ravenna`: RAVENNA discovery, SDP exchange, and PipeWire RTP
  session lifecycle for configured sources and sinks, gated by PTP lock.

### Modified Capabilities

- `os-image-build`: the `qdl` image ships the daemon as an enabled
  `obake` user service and a one-time home-directory config seed. Other
  boards omit the daemon. The seeded `ptp4l` configuration exposes a
  management socket `obake` can open.

## Impact

- New Cargo project at `services/obake-ravenna/`. The `qdl` image packages that
  tree. No shape changes, so portable workloads are unaffected.
- `qdl` Buildroot fragment and a user unit. The shared base does not gain
  the package. Avahi is already on the base image; `obake` needs a
  system-bus policy that allows it to browse and publish.
- OTA delivers the binary and the enabled user unit. An existing home
  directory is left alone except for seeding a missing config. The seed
  defines no streams, so an update does not start transmitting.
- The daemon does not retune PipeWire, open ALSA devices, or discipline
  the PHC. `phc2sys` stays independent of it.
- QEMU cannot exercise a RAVENNA peer or a PTP lock. Image checks cover
  presence, the user unit, and the empty seed. Interoperation with a
  real device is a board test.
