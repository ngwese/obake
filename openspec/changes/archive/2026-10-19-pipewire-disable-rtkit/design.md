# Design

## Context

See proposal.md for motivation. Current state that shapes the approach:

- `obake-pipewire` builds with `-Dlegacy-rtkit=false`. That removes the old
  SPA RTKit path only. `libpipewire-module-rt` still defaults to
  `rtkit.enabled = true` and `rtportal.enabled = true`, and probes D-Bus at
  startup even when rlimits already allow `SCHED_FIFO`.
- `image/host/systemd/pipewire.service.d/10-obake-rt.conf` sets
  `LimitRTPRIO=95`, `LimitRTTIME=infinity`, and `LimitMEMLOCK=infinity`.
  `wireplumber.service` has no matching drop-in.
- `obake-host-config` installs `image/host/pipewire/pipewire.conf.d/` and
  `image/host/wireplumber/wireplumber.conf.d/` to `/usr/share/...`, and only
  installs systemd drop-ins from `pipewire.service.d/`.
- Linger-started user services do not apply PAM `limits.d`. That path stays
  out of scope (option A).
- Upstream documents `module.rt.args` overrides and the `DISABLE_RTKIT`
  environment variable for the same effect.

## Goals / Non-Goals

**Goals:**

- Stop RTKit and realtime-portal probes from factory config for both
  daemons.
- Give WirePlumber the same unit rlimits PipeWire already has.
- Keep verification and host docs aligned with that behavior.

**Non-Goals:**

- Packaging or vendoring rtkit.
- Changing `@audio` / `limits.d`, PAM, or login-session grants.
- Bumping PipeWire or WirePlumber versions.
- Asserting absence of RTKit log lines in QEMU (unit and factory config
  checks are enough).

## Decisions

### Factory `module.rt.args`, not `DISABLE_RTKIT`

Add:

- `image/host/pipewire/pipewire.conf.d/10-obake-rt.conf`
- `image/host/wireplumber/wireplumber.conf.d/10-obake-rt.conf`

with:

```
module.rt.args = {
  rtkit.enabled = false
  rtportal.enabled = false
}
```

WirePlumber loads PipeWire client modules; the same override key applies.
Prefer config over `Environment=DISABLE_RTKIT=1` so the intent lives next to
the other factory audio drop-ins and remains overridable under
`/home/obake/.config` like clock and ALSA settings.

Alternative considered: only set `DISABLE_RTKIT` on the units. Rejected as
less discoverable and harder for a home-directory override to replace.

### Mirror the PipeWire rlimits drop-in onto WirePlumber

Add `image/host/systemd/wireplumber.service.d/10-obake-rt.conf` with the
same three limits as PipeWire. Extend `obake-host-config.mk` to install
`systemd/wireplumber.service.d/*.conf` the same way it installs
`pipewire.service.d`.

Reuse the filename `10-obake-rt.conf` so both units read the same story.

### Verification checks both units' `LimitRTPRIO`

In `check_pipewire_runtime`, after the existing PipeWire check, also require
`systemctl --user show wireplumber.service -p LimitRTPRIO --value` equals
`95`. Do not parse journal logs for RTKit strings.

### Docs only

Update `image/host/SETUP.md` and the rtkit paragraph in `image/os/README.md`
to mention both units and the factory `module.rt.args` drop-ins.

## Risks / Trade-offs

- [Manual `wireplumber` outside the unit] → Still lacks unit rlimits; factory
  config still disables RTKit probes, so logs stay quiet, but that process
  may not get `SCHED_FIFO`. Mitigate by documenting that the user services
  are the supported path.
- [Home override re-enables RTKit] → Allowed by the existing override model;
  without a daemon the probe would warn again. No mitigation beyond docs.
- [WirePlumber ignores `module.rt.args` in conf.d] → Unlikely on 0.5.18; if
  install-time inspection shows otherwise, fall back to unit
  `Environment=DISABLE_RTKIT=1` in the same change.

## Migration Plan

Factory files ship with the next image build and replace on OTA. No user
data migration. Rollback is booting the previous slot.

## Open Questions

None.
