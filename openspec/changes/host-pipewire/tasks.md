# Tasks

## 1. PipeWire and WirePlumber recipes

- [ ] 1.1 Add `image/os/buildroot/package/obake-pipewire/` (Config.in, `.mk`,
  `.hash`) for PipeWire 1.6.9 with the meson options in design.md, including
  `pipewire-jack` enabled, `jack` and `libpulse` disabled, and
  `session-managers=[]`. Source it from `image/os/buildroot/Config.in`.
  Verify `openspec` is unchanged by this task and the recipe directory lists
  those three files plus any patch that the build actually needs.
- [ ] 1.2 Add `image/os/buildroot/package/obake-wireplumber/` for WirePlumber
  0.5.18 with system Lua, introspection disabled, and the systemd user
  service enabled. Select Buildroot Lua and glib2. Verify the recipe depends
  on `obake-pipewire` and does not select Python or gobject-introspection.
- [ ] 1.3 Point `image/os/buildroot/package/obake-runtime/Config.in` at the two
  new recipes and drop `obake-jack2`. Remove
  `image/os/buildroot/package/obake-jack2/` and its `Config.in` source line.
  Verify the runtime Config.in no longer mentions `OBAKE_JACK2`.
- [ ] 1.4 Record both versions from `image/os/scripts/pinned-inputs.sh` into
  `pinned-inputs.json` in place of `jack2`. Verify a dry run of the script
  against the recipe files emits `pipewire` 1.6.9 and `wireplumber` 0.5.18.
- [ ] 1.5 Document the recipes, the dropped jack2 package, and the absence of
  Python and rtkit in `image/os/README.md` and the `host/` row of
  `image/README.md`. Verify those files name 1.6.9 and 0.5.18 and no longer
  tell the operator to drive `jackdbus`.

## 2. Host configuration

- [ ] 2.1 Replace `image/host/jack/` with PipeWire and WirePlumber factory
  config under `image/host/`: 48000 Hz, quantum 128 fixed, ALSA period size
  128 and period count 3 for `hw:CARD=ADAT`, `hw:CARD=II`, and
  `hw:CARD=USBPre2`, and suspend timeout 0. Do not install an AES67 or
  RAVENNA profile. Update
  `image/os/buildroot/package/obake-host-config/obake-host-config.mk` to
  install that config and a `pipewire.service` user drop-in with
  `LimitRTPRIO=95`, `LimitRTTIME=infinity`, and `LimitMEMLOCK=infinity`.
  Enable `pipewire.service` and `wireplumber.service` on the user default
  target, and do not enable `pipewire-pulse`. Verify the makefile no longer
  installs `/etc/jack` or `jack@.service`.
- [ ] 2.2 Delete `image/host/udev/51-usbpre2.rules`. Keep
  `image/host/udev/50-mixpre6.rules`, `58-push2.rules`, and `59-monome.rules`
  as they are. Verify `51-usbpre2.rules` is gone and the MixPre rule still
  runs `usb_modeswitch` for vendor `0926` product `0206`.
- [ ] 2.3 Update the `obake-linger.service` description, `image/host/SETUP.md`,
  the JACK paragraphs in `image/docs/host-config.md`, and the `host/` row in
  `image/docs/user-guide.md` so they describe the PipeWire user services, the
  runtime socket, and home-directory overrides under `/home/obake/.config`.
  Verify those docs no longer present `/etc/jack` as the way to select a
  sound card.

## 3. Headless verification

- [ ] 3.1 Extend `image/verification/guest/obake-verify.sh` so a booted guest,
  with no audio device, fails unless the `obake` user manager has PipeWire
  and WirePlumber active, the PipeWire socket exists under
  `/run/user/<obake-uid>/`, the PipeWire user service shows realtime priority
  95, `jackd` is absent, no `pipewire-pulse` unit is installed, and
  `50-mixpre6.rules` is installed. Verify by reading the new assertions in
  that script; the QEMU run is the integration check in 3.2.
- [ ] 3.2 Run the existing headless QEMU guest verification and confirm the
  new assertions pass. Verify the command's exit status is success.
