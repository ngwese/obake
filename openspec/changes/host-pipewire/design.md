# Design

## Context

See proposal.md for motivation. Behavior that shapes the approach:

- `obake-jack2` builds jack2 1.9.22 (`--alsa --classic --dbus`) and deletes
  `jack_control` so the host has no Python. `obake-runtime` selects it.
  `obake-host-config` installs `image/host/jack/*.conf` to `/etc/jack` and
  `jack@.service`. Profiles differ: MixPre and USBPre are `-n3 -p256 -r48000`,
  ADAT is `-p128`, AES67/RAVENNA is `-n6 -p48`.
- `50-mixpre6.rules` runs `usb_modeswitch` for vendor `0926` product `0206`.
  `51-usbpre2.rules` runs `systemctl start jack@usbpre` from udev, which does
  not work. Push 2 and monome rules only set `MODE="0666"`.
- `obake` is in group `audio`. `@audio` has `rtprio` 95 and unlimited
  `memlock`. `jack@.service` repeats those limits on the unit because a
  linger-started user manager does not apply PAM `limits.d`. Buildroot has no
  rtkit package; the image documents that omission.
- `obake-linger.service` starts `user@<uid>` at boot. A session bus is already
  installed for that user.
- `obake` cannot write `/etc`. Home is `/home/obake` on the user partition.
- Buildroot 2026.02.3 packages PipeWire 1.2.8 and WirePlumber 0.5.10. Its
  PipeWire recipe enables `pipewire-jack` only when in-tree `jack2` is
  selected, and that switch also builds the plugin that makes PipeWire a
  client of `jackd`. Debian Trixie ships PipeWire 1.4.2. A 1.4 JACK client
  writes UMP on the wire; a 1.2.8 server has no UMP type and drops those
  events. The interface versions a 1.4.2 client binds are the same as 1.2.8,
  so audio connects, and a 1.6 server accepts the UMP a 1.4 client sends.
- WirePlumber 0.5.18 requires `libpipewire` >= 1.0.2 and glib >= 2.68.
  Buildroot already has glib 2.86.5, alsa-lib 1.2.15.3, and Meson 1.10.1.
  PipeWire 1.6.9 requires alsa >= 1.2.6 and Meson >= 0.61.1.

## Goals / Non-Goals

**Goals:**

- One graph, started at boot as `obake`, that opens ALSA cards as they appear.
- Realtime scheduling from the user unit's rlimits.
- A socket in the `obake` runtime directory that a same-uid container can bind.
- ALSA-sequencer MIDI without a JACK server.
- Pins and hashes so the audio stack does not float with Buildroot.

**Non-Goals:**

- Rebuilding shapes. That is `trixie-jack-shapes`.
- PulseAudio, Bluetooth, video, or a JACK server kept beside PipeWire.
- A per-device quantum. One graph has one quantum.
- The AES67/RAVENNA profile (`hw:CARD=RAVENNA`, 6 periods, 48 frames). That
  returns in a later change. A RAVENNA card that appears anyway is opened
  like any other ALSA card, at this graph clock, and is not given its old
  profile.
- QEMU coverage of USB attach, ADAT, MixPre audio, or MIDI. Those need
  hardware.

## Decisions

### External recipes at 1.6.9 and 0.5.18

Add `obake-pipewire` and `obake-wireplumber` next to `obake-jack2`, then
remove `obake-jack2`. `obake-runtime` selects the new recipes. In-tree
`pipewire` and `wireplumber` stay unselected so a Buildroot bump cannot
change the version.

Each recipe is a meson package with `INSTALL_STAGING = YES`, a GitLab tag
archive, and a `.hash` that covers the tarball and the license files
(`BR2_DOWNLOAD_FORCE_CHECK_HASHES`). `pinned-inputs.sh` records
`pipewire` and `wireplumber` in place of `jack2`.

WirePlumber selects Buildroot Lua (5.3 or 5.4) and glib2. PipeWire selects
alsa-lib with sequencer, raw MIDI, and UCM, plus dbus and systemd. Neither
recipe selects Python. `BR2_TOOLCHAIN_BUILDROOT_CXX` stays; this change does
not need it, and removing it is a separate toolchain question.

PipeWire 1.6 meson defaults differ from the 1.2 recipe. The `.mk` sets the
options that matter:

- enabled: `pipewire-jack`, `pipewire-alsa`, `alsa`, `udev`, `dbus`,
  `systemd`, `systemd-user-service`
- disabled: `jack` (the client-of-jackd plugin), `libpulse`, `flatpak`,
  `gstreamer`, `bluez5`, `v4l2`, `vulkan`, `legacy-rtkit`, docs, man, tests,
  examples
- `session-managers=[]` so PipeWire does not fetch WirePlumber as a subproject

WirePlumber: `system-lua=true`, `introspection=disabled`, `systemd=enabled`,
`systemd-user-service=true`, `elogind=disabled`, docs and tests off.

`pipewire-jack` defaults to enabled in 1.6 and does not need system jack
headers. `jack=auto` would turn on if jack2 were installed, so it is set
disabled explicitly.

Alternative considered: stay on Buildroot's PipeWire 1.2.8. Rejected because
the Trixie 1.4.2 client, which the companion change uses, sends JACK MIDI as
UMP, and 1.2.8 drops it. The supported protocol direction is an older client
against a newer server.

### One clock, taken from the ADAT profile

`image/host/jack/adat.conf` is `-n3 -p128 -r48000` on `hw:CARD=ADAT`. Factory
config keeps that clock for the whole graph: `default.clock.rate` 48000,
`allowed-rates` 48000 only, and `min-quantum` = `max-quantum` = `quantum` =
128. WirePlumber opens `hw:CARD=ADAT`, `hw:CARD=II` (MixPre), and
`hw:CARD=USBPre2` with `api.alsa.period-size` 128 and `api.alsa.period-num`
3, and sets `session.suspend-timeout-seconds` 0 on every card.

MixPre and USBPre move from 256 frames to 128. The AES67 profile in
`image/host/jack/aes67.conf` is not installed. A single graph cannot give
RAVENNA its 48-frame, 6-period clock while these three cards run at 128.

`obake` can override either file under
`/home/obake/.config/pipewire/` and `/home/obake/.config/wireplumber/`
without sudo. Factory files are installed by `obake-host-config` from
`image/host/` onto the image (read-only root, replaced by OTA).

### Realtime on the user unit

Install a drop-in for the PipeWire user service:

```
LimitRTPRIO=95
LimitRTTIME=infinity
LimitMEMLOCK=infinity
```

Build with `rlimits-install=false`. Upstream's limits file matches group
`pipewire`, caps memlock at 4 GB, and would not apply to a linger-started
service anyway. `@audio` in `obake-audio.conf` stays as the login-session
grant. Do not install rtkit. `legacy-rtkit=false` keeps that module out;
without a daemon the realtime module uses the rlimits above.

Enable `pipewire.service` and `wireplumber.service` on the user
`default.target`. `obake-linger.service` already starts the user manager.
Do not enable `pipewire-pulse`.

### Hotplug and the MixPre rule

WirePlumber's ALSA monitor is the hotplug path. Delete
`51-usbpre2.rules`. Keep `50-mixpre6.rules` unchanged: the mode switch runs
on USB add, the device re-enumerates as UAC2, and the monitor opens the new
card. Keep `58-push2.rules` and `59-monome.rules`.

ALSA sequencer support is turned on with the alsa-lib seq option. PipeWire
opens raw MIDI devices and the sequencer bridge publishes them on
`/dev/snd/seq`. Container clients that already see `/dev` do not need
PipeWire libraries for sequencer MIDI.

### What QEMU can check

Extend the existing guest functional checks, which already run as `obake`
and already know the user bus. Assert active user units, the socket path,
`LimitRTPRIO=95`, no `jackd`, no `pipewire-pulse` unit, and the MixPre rule
file contents. Do not claim a USB device was opened.

## Risks / Trade-offs

- [One quantum] → ADAT stays at 128 frames. MixPre and USBPre move from 256
  to 128, which raises their interrupt rate. AES67 is not given its 48-frame
  clock in this change. A home-directory override can pin a different
  quantum, still for the whole graph.
- [MixPre re-enumeration race] → The mode switch runs before the UAC2 card
  exists. If a UAC1 card appears first, the monitor must drop it when the
  device re-enumerates. Confirm on hardware; QEMU cannot.
- [Meson option drift on the next bump] → A renamed flag fails configure, and
  a new default-on feature (as `flatpak`, `rlimits-install`, and
  `session-managers` did in 1.6) silently grows the image. Diff
  `meson_options.txt` at each pin bump.
- [glib2 on the image] → WirePlumber needs it. Introspection stays off so
  Python stays off. The image gets libffi, pcre2, and a small Lua
  interpreter.
- [`/etc/jack` left in an overlay upper] → Nothing reads it after this
  change. It is inert. The other slot still boots the previous image.

## Migration Plan

1. Land the recipes, host config, and udev edit. Build and confirm the QEMU
   guest checks.
2. Refresh `vendor/dl` with `vendor.sh` so the two tarballs are in git-lfs
   and the jack2 source is no longer required to build.
3. OTA or reinstall picks up the new slot. Rollback is the previous slot,
   which still has `jackd`.
4. Shape cutover is the companion change. Until those images are rebuilt,
   JACK clients in existing bookworm shapes do not connect.

## Open Questions

None. The quantum is fixed at 128 by the spec; changing it is a spec change.
