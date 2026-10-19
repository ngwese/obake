# Proposal

## Why

PipeWire and WirePlumber still probe RTKit over D-Bus at startup. Buildroot
ships no rtkit package, so every boot logs `ServiceUnknown` warnings even
though the image already grants realtime via systemd user-service rlimits.
Those probes should be turned off, and WirePlumber should get the same
rlimits PipeWire already has.

## What Changes

- Factory PipeWire and WirePlumber config disables RTKit and the realtime
  portal in `libpipewire-module-rt` (`rtkit.enabled` / `rtportal.enabled`).
- The WirePlumber user service gets the same realtime rlimits drop-in as
  PipeWire (`LimitRTPRIO=95`, `LimitRTTIME=infinity`, `LimitMEMLOCK=infinity`).
- Guest verification checks WirePlumber's rlimits the same way it already
  checks PipeWire's.
- Docs describe the dual-unit rlimits and the no-RTKit factory config.
- No new packages, no PAM/`limits.d` changes, no PipeWire or WirePlumber
  version bumps.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `host-audio`: Realtime scheduling without rtkit now covers both user
  services and requires factory config that does not use RTKit or the
  realtime portal.
- `image-verification`: Post-boot PipeWire checks also assert WirePlumber
  realtime limits.

## Impact

- `image/host/pipewire/pipewire.conf.d/` — new factory drop-in for
  `module.rt.args`.
- `image/host/wireplumber/wireplumber.conf.d/` — same for WirePlumber.
- `image/host/systemd/wireplumber.service.d/` — new rlimits drop-in;
  `obake-host-config` installs it.
- `image/verification/guest/obake-verify.sh` — assert WirePlumber
  `LimitRTPRIO`.
- `image/host/SETUP.md` and `image/os/README.md` — document the above.
- Factory files live on the read-only image and replace on OTA; home
  overrides under `/home/obake/.config` are unchanged.
- Shape portability and the PipeWire socket path are unaffected.
