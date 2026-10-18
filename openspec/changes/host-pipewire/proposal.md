# Proposal

## Why

The host starts a per-device `jackd` and expects containerized JACK clients to
speak the classic JACK socket protocol. Hotplug is a udev rule that starts one
of those servers, which does not work from udev, and each interface carries its
own period size. PipeWire with WirePlumber can own the devices, follow USB
attach, and expose one graph to ALSA, JACK, and ALSA-sequencer clients.

## What Changes

- **BREAKING**: Replace `obake-jack2` (`jackd`, `jackdbus`, `/etc/jack`
  profiles, `jack@.service`) with pinned external recipes `obake-pipewire`
  1.6.9 and `obake-wireplumber` 0.5.18. The image no longer runs a JACK server.
- Run PipeWire and WirePlumber as `obake` user services, started at boot by the
  existing linger user manager, with realtime scheduling from rlimits. The
  graph clock is 48000 Hz and 128 frames, with three ALSA periods, for the
  ADAT card (`hw:CARD=ADAT`), the MixPre (`hw:CARD=II`), and the USBPre
  (`hw:CARD=USBPre2`). The AES67/RAVENNA profile is not part of this change.
  No PulseAudio compatibility and no rtkit.
- Let WirePlumber open ALSA cards when they appear. Keep the MixPre
  `usb_modeswitch` udev rule so that device enumerates as UAC2. Remove the
  USBPre rule that starts `jack@usbpre`. Leave the Push 2 and monome permission
  rules in place.
- Bridge MIDI through the kernel ALSA sequencer so clients that open
  `/dev/snd/seq` see PipeWire-managed ports.
- Publish the PipeWire socket in the `obake` user runtime directory. Factory
  config ships on the read-only image; per-user overrides live under
  `/home/obake/.config`.
- Record the PipeWire and WirePlumber versions in `pinned-inputs.json` and
  verify the user services headlessly in QEMU.

## Capabilities

### New Capabilities

- `host-audio`: the host audio and MIDI runtime for the `obake` user, including
  device hotplug, the low-latency clock, realtime scheduling, the PipeWire
  socket, and ALSA-sequencer MIDI.

### Modified Capabilities

- `image-verification`: after boot, confirm the PipeWire user services and
  socket, the MixPre udev rule, and that `jackd` is not installed.

## Impact

- `image/os/buildroot/package/obake-pipewire/`, `obake-wireplumber/`, removal
  of `obake-jack2`, and `obake-runtime` / `obake-host-config` selection.
- `image/host/jack/` is replaced by PipeWire and WirePlumber drop-ins and
  systemd user drop-ins. `image/host/udev/50-mixpre6.rules` stays;
  `51-usbpre2.rules` goes.
- `image/os/scripts/pinned-inputs.sh`, `image/os/README.md`,
  `image/README.md`, `image/docs/host-config.md`, `image/host/SETUP.md`.
- Vendor download cache gains the two source tarballs and drops the jack2
  fetch. OTA replaces the factory audio config; `/home/obake` overrides
  survive updates. Existing shapes built against `jackd` do not connect until
  `trixie-jack-shapes` (or an equivalent rebuild) points them at PipeWire's
  `libjack`.
