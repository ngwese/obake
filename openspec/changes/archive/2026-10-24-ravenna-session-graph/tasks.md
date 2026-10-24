# Tasks

## 1. Daemon config

- [x] 1.1 Replace per-source `destination` and `frame_size` with
  `[graph]` (`rate`, `node_group`), `[packet]` (`frames`), and
  `[multicast]` (`address`, `base_port`, `ttl`) in
  `services/obake-ravenna`. `frames` is RTP frames per packet, not the
  PipeWire quantum. Verify `cargo test -p obake-ravenna` covers the
  schema, default TTL 15, and rejection of a missing base address.
- [x] 1.2 Assign `base_port + 2 * index` and the shared address, honor
  optional per-source `address`, `port`, and `payload`, and verify
  tests reject an odd base or pinned port, a non-239 address, the SAP
  group, and two sources that resolve to the same address and port.
- [x] 1.3 Update the seeded `config.toml` comments under the image
  package and `services/obake-ravenna/examples/99-ravenna-test.conf`
  so they match the schema, and verify the seed still defines no
  streams.
- [x] 1.4 Add `services/obake-ravenna/examples/config.toml` for a
  typical setup: 48000 Hz, 48 frames per packet, node group
  `obake.ravenna`, multicast `239.1.60.50` from port 5004, one
  8-channel L24 source, and one sink. Verify a unit test parses that
  file and the image seed still defines no streams.

## 2. SDP, payload type, and grandmaster

- [x] 2.1 Build `a=ts-refclk` from the `ptp4l` grandmaster identity and
  domain in `services/obake-ravenna/src/sdp.rs`, and verify a unit
  test rejects an all-zero identity and checks
  `ptp=IEEE1588-2008:<id>:<domain>`.
- [x] 2.2 Read `PARENT_DATA_SET` grandmaster identity on the existing
  in-process management client in `services/obake-ravenna/src/ptp.rs`,
  and verify a unit test parses a fixture with no child process.
- [x] 2.3 Default the payload type to `96 + channels`, clamp overrides
  to 96..=127, and put that value in both SDP and the RTP args.
  Verify tests for 2 channels (98), 8 channels (104), and an
  out-of-range override holding the source down.
- [x] 2.4 Hold a source down when audio payload
  `channels * frames_per_packet * bytes_per_sample` exceeds 1440
  bytes (8-channel L24 at 64 frames per packet is 1536) and allow
  8-channel L24 at 48 frames per packet (1152). Verify both with unit
  tests. Do not change the NIC MTU.

## 3. DNS-SD path

- [x] 3.1 Publish TXT `path=/by-name/<stream>` on each session
  registration in `services/obake-ravenna/src/avahi.rs`, and verify
  the recording client captures that TXT and the node registration
  stays a `_ravenna` subtype without it.
- [x] 3.2 Compare the TXT key with one live Merging or Anubis session
  (`avahi-browse -r`) and, if the key differs, change only the TXT.
  Verify the recording test matches the key that browse showed.
  (Live Anubis/Merging sessions advertise empty TXT; we still publish
  `path=/by-name/<stream>` so peers that read it can fill RTSP HOST.)

## 4. PipeWire session manager

- [x] 4.1 Extend `PipewireClient` so reconcile can ensure one PTP
  driver and load or unload RTP modules, and verify
  `RecordingPipewire` tests create the driver once, skip
  `object.export`, `aes67.driver-group`, and any quantum override, and
  put `rtp.framecount` and `rtp.payload` on the stream properties.
- [x] 4.2 Implement the production client against the `obake`
  PipeWire socket in `services/obake-ravenna`: one
  `support.node.driver` on `clock.interface` from `ptp4l`, RTP sink
  and source modules in that node group, update by replace, delete on
  disqualify or shutdown. Verify `cargo test -p obake-ravenna` still
  passes without a live daemon, and a board reload of config does not
  restart PipeWire.
- [x] 4.3 Log create, update, and delete at info with the object name,
  and log the applied property set at debug. Verify a unit test on
  the recording client records those actions.

## 5. Qualify and status

- [x] 5.1 Gate transmit and receive on the configured rate and frames
  per packet, PTP lock, and a known grandmaster. Leave the PipeWire
  quantum unchanged when frames per packet differs from it. Verify
  qualify tests for an illegal packet size, a 128-sample quantum with
  48-frame packets still qualifying, a missing grandmaster, and lock
  loss removing RTP nodes while the driver remains.
- [x] 5.2 Keep `obake-ravenna status` reporting lock state and each
  stream's hold-down reason, including MTU and payload-type failures.
  Verify the status test output names those reasons.
