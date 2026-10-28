# Tasks

## 1. PipeWire 1.6.10 pin

- [x] 1.1 Set `OBAKE_PIPEWIRE_VERSION` to `1.6.10` in
  `image/os/buildroot/package/obake-pipewire/obake-pipewire.mk`, refresh
  `obake-pipewire.hash` from the 1.6.10 release tarball (and license
  digests if changed), and re-check that
  `0001-rtp-sink-quantize-timestamps.patch` still applies; verify with a
  package fetch/configure (or documented hash check) that Meson accepts
  the existing `CONF_OPTS`
- [x] 1.2 Update version mentions in `image/docs/user-guide.md` (and any
  other image docs that hard-code 1.6.9) to 1.6.10; verify those docs
  match the `.mk` pin

## 2. Factory UMP enablement

- [x] 2.1 Add a WirePlumber factory drop-in under
  `image/host/wireplumber/wireplumber.conf.d/` that sets
  `monitor.alsa-midi.properties` / `api.alsa.seq.ump = true`, installed
  by the existing host-config path; verify the file is present in a
  built rootfs or staged host tree and that WirePlumber accepts the
  property (no parse error on start)
- [x] 2.2 Document the UMP default and home-override escape hatch in
  `image/docs/user-guide.md` (or host-audio docs linked from it); verify
  the documented property name matches the drop-in

## 3. Image stamp and headless check

- [x] 3.1 Bump `image/os/buildroot/VERSION` for the new runtime and
  verify the file contents match the chosen semver
- [x] 3.2 Run a headless verification available in-tree (package rebuild
  and/or guest `obake-verify` PipeWire checks after an image build) and
  confirm PipeWire reports 1.6.10 with WirePlumber still 0.5.18
