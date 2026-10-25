# Proposal

## Why

After boot, `obake-ravenna` can treat `ptp4l` `SLAVE` as locked and bring
sources up while the PHC is still converging. Peers then hear distorted
audio (ring-modulation-like) until the daemon or stream is restarted once
the clock has settled. Source bring-up needs a hold after lock so media
nodes wait for a stable SLAVE, not only the first rising edge.

## What Changes

- Add a configurable dwell time after PTP lock becomes true: sources (and
  sinks gated the same way) stay held down until `locked` has remained
  continuously true for that duration; any unlock resets the timer.
- Default the dwell to a non-zero value suitable for typical AES67 boot
  settle, with `0` preserving today's immediate bring-up on SLAVE.
- Expose the setting in `config.toml` and surface hold-vs-locked clearly
  in status (so operators can tell "unlocked" from "locked but holding").
- Document the setting in the RAVENNA reference / seed config comments.
- **Not in scope:** servo offset thresholds or other PHC stability metrics
  (considered; deferred for simplicity — see design).

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `obake-ravenna`: PTP lock gating gains a continuous lock-hold before
  sources and sinks may come up; status distinguishes hold from unlock.

## Impact

- `services/obake-ravenna` (config, PTP/qualify path, status text, tests).
- Persistent config under `/home/obake/.config/obake-ravenna/` (new key;
  absent key uses the default).
- No image/OTA or shape changes; behavior is daemon-only and portable with
  the config file.
