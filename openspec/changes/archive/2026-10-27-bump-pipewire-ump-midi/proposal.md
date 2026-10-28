# Proposal

## Why

The image still ships PipeWire 1.6.9. The latest 1.6.x release is 1.6.10
(maintenance fixes). Separately, mid-1.6.x stopped preferring UMP on the
graph and again treats MIDI 1.0 as the default transport; MIDI 2.0 / UMP
on the ALSA sequencer must be opted in. The qdl kernel already enables
USB MIDI 2.0, so the host runtime should request UMP by default so those
devices are usable without per-host WirePlumber overrides.

## What Changes

- Bump the pinned `obake-pipewire` package from 1.6.9 to 1.6.10 (version,
  source hash, docs, and pinned-input recording).
- Enable UMP / MIDI 2.0 packet handling by default on the ALSA MIDI
  bridge via factory WirePlumber config (`api.alsa.seq.ump = true`).
- Leave WirePlumber at 0.5.18 unless 1.6.10 requires a newer session
  manager (not expected).
- Bump `image/os/buildroot/VERSION` so OTA bundles carry the new runtime.
- Update `host-audio` (and any verification text that hard-codes 1.6.9).

## Capabilities

### New Capabilities

- (none)

### Modified Capabilities

- `host-audio`: pin PipeWire 1.6.10; require factory config that enables
  UMP on the ALSA sequencer MIDI bridge by default.

## Impact

- `image/os/buildroot/package/obake-pipewire/` (`.mk`, `.hash`).
- `image/host/wireplumber/wireplumber.conf.d/` (new or extended drop-in).
- Docs that name 1.6.9 (`image/docs/user-guide.md`, related READMEs).
- `image/os/buildroot/VERSION` (minor or patch bump for the shipped OS).
- OTA: hosts need a rebuilt image / update bundle; home overrides under
  `/home/obake/.config` remain and can still override the factory UMP
  setting.
- Shapes: no pin bump required (they talk JACK/PipeWire IPC; host carries
  the daemon version).
