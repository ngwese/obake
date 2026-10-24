# Proposal

## Why

Peers mute obake's RAVENNA stream and leave CLOCK DOMAIN and RTSP HOST
blank. The served SDP names a zero PTP clock identity, the RTP payload
type does not match that SDP, session advertisements carry no path, and
the daemon never creates the PTP graph driver or the RTP nodes. A
hand-written PipeWire drop-in cannot track config reloads.

## What Changes

- obake-ravenna becomes the session manager for one RAVENNA graph on
  the existing PipeWire daemon, beside WirePlumber. It creates, updates,
  and removes the PTP driver and each RTP source and sink. It does not
  restart PipeWire and does not retune the ALSA clock.
- **BREAKING** for `add-obake-ravenna` config: sample rate and RTP
  frames per packet are daemon settings, and they are not the PipeWire
  quantum. The quantum stays whatever PipeWire already uses. Transmit
  multicast address is one base address. Ports are
  `base_port + 2 * source index` in file order. Per-source
  `destination` is no longer required.
- SDP `a=ts-refclk` carries the live grandmaster clock identity and
  domain from `ptp4l`. A zero identity is not advertised.
- Transmit payload type defaults to `96 + channels`, with an optional
  per-source override, and that value is what both SDP and RTP use.
- Session DNS-SD records include `path=/by-name/<stream>` so Merging and
  Anubis can set RTSP HOST.
- Create, update, and delete of a PipeWire object is logged at info.
  The full object configuration is logged at debug.
- This change takes precedence over conflicting decisions in
  `add-obake-ravenna`. Do not sync that change's `obake-ravenna` delta
  into main specs. This change is the spec of record for that
  capability.

## Capabilities

### New Capabilities

- `obake-ravenna`: RAVENNA session management on the host PipeWire
  daemon, including the PTP graph, stream lifecycle, SDP, and DNS-SD.

### Modified Capabilities

## Impact

- `services/obake-ravenna/` config schema, SDP builder, Avahi TXT,
  PipeWire client (replacing the recording stub for production), and
  reconcile loop.
- Persistent config at `/home/obake/.config/obake-ravenna/config.toml`
  changes shape. Existing per-source `destination` and `frame_size`
  fields stop being the source of truth. The installed seed stays an
  empty stream list.
- `services/obake-ravenna/examples/config.toml` shows a typical setup:
  48 kHz, 48 frames per packet, one 8-channel L24 source.
  `examples/99-ravenna-test.conf` stays a PipeWire property checklist,
  not the runtime config.
- No OTA or shape-image change. PipeWire and `ptp4l` stay separate
  processes.
