# Tasks

## 1. Stream rules

- [x] 1.1 Create `services/obake-ravenna/` as a Cargo project (library plus a
  `obake-ravenna` binary) and verify `cargo test` runs with no tests
  failing
- [x] 1.2 Parse `config.toml` sources and sinks (`name`, `channels`,
  `format`, `frame_size`, `destination`, optional `ttl`, sink `device`
  and `stream`) and verify unit tests accept the design's example and
  reject an unknown format
- [x] 1.3 Implement frame-size, destination, port, TTL, and duplicate
  checks (6, 12, 16, 32, 48, 64, 192 samples at 48000 Hz; `239.0.0.0/8` except
  `239.255.255.255`; even port; default port 5004; default TTL 15;
  both duplicates held down) and verify unit tests cover each case
- [x] 1.4 Implement the pure qualify step (config, graph rate, lock,
  interface present, discovered SDP) that emits hold-down reasons
  `unlocked`, `bad-frame-size`, `bad-destination`,
  `duplicate-destination`, `interface-missing`, `device-absent`,
  `unicast-sdp`, `rate-mismatch`, and `domain-mismatch`, and verify
  unit tests for a legal source, a 64-sample source, an unlocked
  clock, a unicast SDP, a rate mismatch, and a domain mismatch
- [x] 1.5 Document the config fields and hold-down reasons in
  `services/obake-ravenna/README.md` and verify the documented example
  matches the parser test

## 2. SDP, discovery, and status

- [x] 2.1 Build and parse source and sink SDP (`rtpmap`, `ptime`,
  `framecount`, connection address, port, `ts-refclk` domain) and
  verify unit tests round-trip a 48-sample L24 source at 48000 Hz
  onto group `239.69.1.10` port 5004
- [x] 2.2 Serve DESCRIBE at `/by-name/<stream>` from the current
  qualify result, with the path in one function, and verify a test
  client receives SDP for a transmitting source and no SDP for a
  held-down source
- [x] 2.3 Browse and publish `_ravenna._sub._rtsp._tcp` through Avahi
  on the system bus, on the `ptp4l` interface only, and verify a
  unit test of the publish call uses that service type and interface
  index rather than opening UDP 5353
- [x] 2.4 Read `ptp4l` lock and domain in-process over the read-only
  management Unix socket with IEEE 1588 management TLVs
  (`GET PORT_DATA_SET`, `GET DOMAIN`), keep the socket open across
  ticks, and do not spawn `pmc` or any other helper; verify unit
  tests against captured binary replies map SLAVE to locked and any
  other port state to unlocked, and that the read path creates no
  child process
- [x] 2.5 Add `obake-ravenna status` over a socket in the user
  runtime directory and verify it prints the lock state and each
  stream's hold-down reason for the qualify-step fixtures

## 3. PipeWire nodes

- [x] 3.1 Load and unload `libpipewire-module-rtp-sink` and
  `libpipewire-module-rtp-source` on the existing `obake` PipeWire
  socket, mapping frame size to `sess.latency.msec`, L16/L24 to
  `S16BE`/`S24BE`, and the join or send interface through
  `IP_MULTICAST_IF`, and verify a fake client records load and unload
  by configured name when qualify results change
- [x] 3.2 Read `clock.rate` from the running graph and refuse to
  change rate or quantum, and verify a test supplies 48000 and the
  qualify step uses that rate rather than a config field
- [x] 3.3 Reconcile nodes on `SIGHUP` without restarting PipeWire, and
  verify a test that swaps a sink's SDP group unloads the previous
  name before loading the new one
- [x] 3.4 Confirm the 1.6.9 module accepts the PTP reference-clock
  argument, and if it does not, pass the module's PTP clock token
  while the served SDP still carries the domain; verify the SDP unit
  test from 2.1 still reports the domain

## 4. qdl image

- [x] 4.1 Add `image/os/buildroot/package/obake-ravenna/`
  (Config.in, makefile, local site `services/obake-ravenna/`) using the
  pinned Buildroot cargo infrastructure, link it to `obake-pipewire`
  staging, and set the package version from the crate; verify the
  makefile does not change `OBAKE_PIPEWIRE_VERSION` or
  `OBAKE_WIREPLUMBER_VERSION` and that no file under `shapes/` changes
- [x] 4.2 Select the package only in
  `image/os/buildroot/configs/obake-qdl.config`, source its Config.in
  from `image/os/buildroot/Config.in`, and leave
  `image/os/buildroot/package/obake-runtime/Config.in` and
  `configs/obake_base.config` unselected; verify
  `image/os/buildroot/check-boards.sh` still passes
- [x] 4.3 Install the user unit
  `obake-ravenna.service` (`After=pipewire.service`,
  `WantedBy=default.target`) and a system-bus policy allowing `obake`
  to browse and publish on Avahi; verify the unit is not a system
  unit and that `image/host/` is unchanged
- [x] 4.4 Install the empty seed at
  `/usr/share/obake/obake-ravenna/config.toml` and a type-`C`
  tmpfiles snippet for `/home/obake/.config/obake-ravenna/`, and
  verify the seed defines no sources and no sinks
- [x] 4.5 Add `RuntimeDirectory=ptp4l` to
  `image/os/buildroot/board/obake/qdl/rootfs-overlay/usr/lib/systemd/system/ptp4l.service`
  and `uds_address` plus `uds_ro_address` under `/run/ptp4l` in
  `image/os/buildroot/board/obake/qdl/rootfs-overlay/usr/share/obake/linuxptp/ptp4l.cfg`;
  verify the AES67 client keys remain and the unit is still not
  enabled by `50-obake-linuxptp.preset`
- [x] 4.6 Document, in `image/docs/user-guide.md` next to the PTP
  section, the user service, the config fields, and the two
  `uds_*` keys an existing `ptp4l.cfg` needs; verify the new section
  names `obake-ravenna` and `/home/obake/.config/obake-ravenna/`

## 5. Headless image check

- [x] 5.1 Extend `image/verification/guest/obake-verify.sh` so a `qdl`
  boot requires `obake-ravenna` on `PATH`, the user service active
  with no login, a home config that defines no streams, and `ptp4l`
  inactive, and every other board requires the binary to be absent;
  verify the script reports those checks by name
