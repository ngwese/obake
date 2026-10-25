# Proposal

## Why

When the AES67 grandmaster disappears for a long time and returns,
`ptp4l` can report FOLLOWER while the PHC is still seconds off. With
default `step_threshold 0.0`, the servo only slews (often pegged at
`max_frequency`), so RMS stays in the billions of nanoseconds for hours.
`obake-ravenna` already treats FOLLOWER + lock-hold as "ready" and brings
RTP nodes up into that broken clock. Restarting `ptp4l` "fixes" it only
because `first_step_threshold` allows a jump on the first update after
start. Media bring-up needs an offset gate, and the seeded client profile
needs to allow steps after GM reacquisition.

## What Changes

- Poll `ptp4l` offset over the existing read-only management socket each
  tick (same path as lock/domain/GMID; still no `pmc` child).
- Hold sources and sinks down while `|offset|` exceeds a configurable
  nanosecond threshold; distinct status reason from unlock / lock-hold.
- Default the threshold to a non-zero value that blocks multi-second
  offsets; `0` disables the offset gate (lock-hold alone remains).
- Seed `step_threshold` in the qdl `ptp4l.cfg` client profile so a large
  post-GM offset jumps the PHC instead of slewing for hours. Existing
  host configs are not rewritten on update (same seed rule as today);
  document the key for operators.
- Document both settings in the RAVENNA reference / user guide.
- **Not in scope:** parsing journal `rms` lines; computing a multi-sample
  RMS in the daemon; restarting or signaling `ptp4l` from the daemon;
  stepping clocks from `obake-ravenna`.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `obake-ravenna`: PTP gating gains an offset threshold before sources
  and sinks may come up; management reads include offset; status
  distinguishes offset-high from unlock / lock-hold.
- `os-image-build`: seeded AES67/RAVENNA `ptp4l` client profile steps the
  clock when offset exceeds a configured threshold (not only on first
  start).

## Impact

- `services/obake-ravenna` (PTP management TLVs, config, qualify/status,
  tests, REFERENCE_GUIDE / README).
- Persistent daemon config under `/home/obake/.config/obake-ravenna/`
  (new `[ptp]` key; absent key uses the default).
- Seeded `ptp4l.cfg` under
  `image/os/.../usr/share/obake/linuxptp/` and user-guide notes; hosts
  that already copied the seed keep their file until edited.
- No shape or OTA protocol changes; portable with host config files.
