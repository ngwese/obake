# Design

## Context

See proposal.md. `add-obake-ravenna` already qualifies streams, serves
RTSP, and reconciles a `PipewireClient` trait, but production still
uses `RecordingPipewire`. SDP hardcodes a zero `ts-refclk` identity
and payload type 97. Avahi publishes an empty TXT. Conflicting
requirements in that change (live graph rate, per-source destination,
"do not touch the graph") lose to this change. Do not archive that
change's `obake-ravenna` delta into main specs.

`examples/99-ravenna-test.conf` is the property checklist. It is not
loaded at runtime.

## Goals / Non-Goals

**Goals:**

- One RAVENNA node group, one PTP driver, RTP nodes created and
  removed through the existing PipeWire socket.
- Config-driven rate and RTP frames per packet, kept independent of
  the PipeWire quantum, plus one base multicast address and an even
  base port.
- SDP, RTP payload type, and session TXT that Merging and Anubis can
  lock and address.

**Non-Goals:**

- Retuning PipeWire `default.clock.*` or WirePlumber ALSA policy.
  The graph quantum is a DSP block, not an RTP packet.
- Raising NIC MTU or sending oversized packets via IP fragmentation.
- Creating the manual `audiotestsrc` tone node.
- A second PipeWire daemon, SAP announcements, or disciplining PTP.
- Linking Ravenna nodes to ALSA. `node.autoconnect` stays false.

## Decisions

1. **Session manager beside WirePlumber.** The daemon is a PipeWire
   client on the `obake` socket. WirePlumber keeps ALSA. Ravenna
   objects use `node.group` from config (default `obake.ravenna`),
   `device.api = aes67`, and `node.autoconnect = false`. Alternative
   considered: keep shipping a `pipewire.conf.d` drop-in. Rejected
   because that file is applied only at PipeWire start, so a config
   reload cannot create or remove streams.

2. **Driver is a factory node; streams are modules.** Ensure one
   `support.node.driver` with `clock.interface` set to the `ptp4l`
   netdev, `priority.driver` high enough to win the group, and no
   `object.export` (export registered a twin node on the board).
   Transmit uses `libpipewire-module-rtp-sink`. Receive uses
   `libpipewire-module-rtp-source`. Do not set `aes67.driver-group`.
   On this PipeWire that flag starts a separate sender whose
   timestamps run about two quanta behind the PHC. `node.group` alone
   is the upstream AES67 pattern. The driver stays while the
   interface is present. Lock loss removes RTP nodes only.

3. **PipeWire arguments that actually take effect.** `rtp.framecount`
   and `rtp.payload` go in `stream.props`. The sink module does not
   copy those keys from the top-level module args. `sess.ts-refclk`
   stays the token `ptp=traceable` with `rtp.fetch-ts-refclk = true`.
   The IEEE grandmaster string is for SDP only. `sess.min-ptime` and
   `sess.max-ptime` bracket the configured packet time.
   `sess.latency.msec` is an integer multiple of that packet time, at
   least one packet. `net.mtu` stays 1500. Do not set
   `clock.force-quantum` or the node's quantum to `rtp.framecount`.
   PipeWire's rtp-sink already accepts a larger cycle and emits
   `rtp.framecount` packets, keeping the remainder. A 128-sample
   quantum and 48-frame packets run together.

4. **Grandmaster identity from the management socket.** Extend the
   existing in-process `ptp4l` client to read `PARENT_DATA_SET`
   `grandmasterIdentity` plus domain. Format
   `ptp=IEEE1588-2008:<EUI-64>:<domain>`. No identity means the source
   stays down. Do not spawn `pmc`.

5. **Payload type `96 + channels`.** Matches the Merging UI (`98` for
   two channels). Optional per-source `payload` in 96..=127 overrides
   it. The same integer is written into SDP `m=` / `a=rtpmap` and
   `rtp.payload`. Receivers copy the remote SDP's payload type.

6. **Address fixed, port stepped.** `[multicast] address` plus
   `base_port + 2 * index` in `[[source]]` file order. Optional
   per-source `address` or `port` pins one stream. Odd `base_port` or
   odd pinned port holds the affected sources down. Duplicate
   address-and-port holds both down. TTL defaults to 15 at
   `[multicast]`.

7. **Payload limit, not quantum.** Payload bytes are
   `channels * frames_per_packet * bytes_per_sample` (L24 is 3).
   RAVENNA's conservative cap on a 1500-byte path is 1440 bytes.
   8-channel L24 at 64 frames per packet is 1536 and stays down.
   8-channel L24 at 48 frames per packet is 1152 and is the
   interoperable default (48 kHz, 1 ms, 1–8 channels). Wider channel
   counts are separate streams, not one fragmented packet. Do not
   change the NIC MTU.

8. **RTSP host.** Node service stays `_rtsp._tcp` plus subtype
   `_ravenna._sub._rtsp._tcp`. Each transmitting session adds subtype
   `_ravenna_session._sub._rtsp._tcp` and TXT `path=/by-name/<name>`.
   Assumption: that TXT key is what Merging and Anubis use to fill
   RTSP HOST. Confirm with one `avahi-browse -r` of a working peer
   during implementation; if the key differs, change the TXT only.

9. **Logging.** Info on each create, update, and delete names the
   object and the action. Debug prints the property set sent to
   PipeWire. The test double records the same events.

10. **Config shape.** `frames` is samples per RTP packet. It is not
    the PipeWire quantum. Checked in as
    `services/obake-ravenna/examples/config.toml`. The image seed
    stays an empty stream list.

    ```toml
    [graph]
    rate = 48000
    node_group = "obake.ravenna"

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
    # payload = 104
    # address = "239.1.60.51"
    # port = 5010

    [[sink]]
    name = "obake-in-1"
    device = "Anubis"
    stream = "Anubis_Combo 1/2"
    ```

    Rate and frames per packet are not per source. Format and
    channels stay per source.

## Risks / Trade-offs

- [Inserting a source renumbers later ports] → Document file order.
  Pin `port` on streams that must not move.
- [TXT key might not be `path`] → Confirm against a live Merging
  record before calling the host gap closed. SDP and payload type do
  not depend on it.
- [libpipewire linkage grows the qdl package] → Already accepted for
  this daemon. Unit tests keep the recording client and do not link
  it.
- [`add-obake-ravenna` still lists the old requirements] → This
  change is the spec of record. Do not sync the older delta.

## Migration Plan

Replace the seeded `config.toml` comments and schema with `[graph]`
and `[multicast]`. On upgrade, a file that still uses per-source
`destination` and `frame_size` fails to parse; the daemon keeps the
last good config and logs the parse error (same as today's SIGHUP
behavior). Roll back by stopping `obake-ravenna`; it removes the
objects it created. Leave any hand-installed `99-ravenna-test.conf`
off the board so it does not duplicate the driver.

## Open Questions

- None that change the spec. The TXT key check in decision 8 is a
  verification step, not a second design.
