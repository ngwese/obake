# Design

## Context

See proposal.md for motivation. Today `obake-ravenna` polls PORT /
PARENT / DOMAIN TLVs and gates on FOLLOWER + `lock_hold_ms`. The
lock-hold design deferred servo offset monitoring as approach A. The
seeded `ptp4l.cfg` has no `step_threshold` (linuxptp default `0.0` =
never step after the first update), so a multi-second PHC error after
GM loss only slews.

## Goals / Non-Goals

**Goals:**

- Gate media nodes on `|offset| <= threshold` using the existing UDS
  management client.
- Keep qualify/status/config local to the daemon; no new processes.
- Seed a PHC `step_threshold` so recovery does not depend on restarting
  `ptp4l`.
- Make offset-hold visible in status separately from unlock / lock-hold.

**Non-Goals:**

- Parsing journal `rms` / `summary_interval` lines.
- Maintaining a multi-sample RMS or variance in the daemon.
- Restarting, signaling, or reconfiguring `ptp4l` from the daemon.
- Stepping or slewing any clock from `obake-ravenna`.
- Rewriting already-seeded host `ptp4l.cfg` on image update.

## Decisions

### 1. Offset source: `TIME_STATUS_NP.master_offset`

**Choice:** GET `TIME_STATUS_NP` and use `master_offset` (signed
nanoseconds).

**Alternatives:**

- `CURRENT_DATA_SET.offsetFromMaster` (IEEE TimeInterval, ÷65536) —
  standards-friendly but more parse work for the same signal.
- Journal `rms` — not available over UDS; fragile.

`TIME_STATUS_NP` is the linuxptp vendor TLV already used by operators
via `pmc` and matches the “last measured offset” intuition. One GET
per tick alongside existing queries.

Failed offset GET while FOLLOWER + non-zero threshold → treat as
offset-high (hold down), not as unlocked (status still shows locked).

### 2. Config: `[ptp].max_offset_ns`

- Type: non-negative integer nanoseconds.
- Default when absent: `1000000` (1 ms). Blocks multi-second disasters
  without demanding sub-µs lock for bring-up; operators can tighten.
- `0`: disable offset gate (lock-hold only), matching the zero-disable
  pattern of `lock_hold_ms`.

Qualify order (existing then new): unlocked → lock-hold →
no-grandmaster / interface / … → **offset-high** → other stream checks.
Offset checked only when locked and hold satisfied, so status reason
priority stays readable.

### 3. Instantaneous threshold, not RMS

**Choice:** each tick compares `|master_offset|` to `max_offset_ns`.

**Alternative:** N consecutive samples under threshold — better for
noisy links, more state and tuning. Defer; 1 ms already ignores normal
AES67 jitter, and the failure mode of interest is seconds of error.

If a single tick spikes above threshold while streams are up, nodes
drop until the next tick(s) are under threshold. Acceptable for
recovering from steps; revisit if flapping appears.

### 4. Seed `step_threshold 0.001`

**Choice:** add `step_threshold 0.001` (1 ms) to the seeded AES67
client `ptp4l.cfg`. Complements the daemon gate: PHC jumps, then
offset falls, then media comes up.

**Not** changing `phc2sys` (system clock still slews only). Document
that existing host configs must add the key manually; tmpfiles `C`
does not overwrite.

### 5. Status / DownReason

Add `DownReason::OffsetHigh` → `"offset-high"`. PTP status line stays
locked/unlocked from port state; per-stream reason shows the hold.
Optionally log current `|offset|` at debug when holding.

## Risks / Trade-offs

- [Default 1 ms too loose] → Audio briefly odd after a step settles
  under 1 ms. Mitigation: document tightening `max_offset_ns`.
- [Default too tight on bad NIC] → Streams stay down after lock.
  Mitigation: raise threshold or set `0`; status shows `offset-high`.
- [Existing hosts lack `step_threshold`] → Offset hold keeps media down
  for hours of slew. Mitigation: user-guide callout; seed for new
  installs.
- [Step while nodes up] → Offset spike drops nodes across the step.
  Desired; avoids transmitting on a discontinuous clock.
- [TIME_STATUS_NP unavailable] → Conservative hold when threshold
  non-zero. Mitigation: unlikely on linuxptp 4.4; tests cover failure.

## Migration Plan

- Ship daemon with default `max_offset_ns`; absent key picks it up.
- Ship seed with `step_threshold`; new/empty config dirs get it.
- Operators on existing hosts: add `step_threshold` to
  `~/.config/linuxptp/ptp4l.cfg` and restart `ptp4l`; optionally tune
  `[ptp].max_offset_ns`.
- Rollback: prior daemon binary, or `max_offset_ns = 0` on a build that
  includes the key; remove/zero `step_threshold` in `ptp4l.cfg`.

## Open Questions

- None that block planning; default `1000000` ns may be adjusted during
  apply from bata experience without changing the gating model.
