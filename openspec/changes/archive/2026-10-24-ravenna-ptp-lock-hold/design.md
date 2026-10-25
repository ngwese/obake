# Design

## Context

See proposal.md for motivation. Today qualify treats `PORT_DATA_SET ==
SLAVE` as locked and may bring RTP sources/sinks up on that edge
(`qualify.rs` + `ptp.rs`). The daemon already polls the read-only
`ptp4l` management socket each tick; it does not read servo offset TLVs.
Config lives in TOML under `/home/obake/.config/obake-ravenna/`.

## Goals / Non-Goals

**Goals:**

- Gate source/sink bring-up on continuous SLAVE for a configured dwell.
- Keep the change local to qualify/status/config; no new processes.
- Make hold visible in status so it is not confused with unlock.

**Non-Goals:**

- Stepping or slewing any clock.
- Changing when `ptp4l` itself is considered SLAVE.
- Measuring PHC/servo stability via offset thresholds in this change.

## Decisions

### Approaches considered

Two approaches were considered for “do not bring sources up until the
media clock is ready enough.”

**A — Servo / PHC stability monitoring**

After `SLAVE`, also poll management data such as `CURRENT_DATA_SET` /
`TIME_STATUS_NP` (`offsetFromMaster`, etc.) and require |offset| under a
threshold for N samples (or a variance bound) before qualify allows
media nodes up.

- Pros: Closer to true AES67 “clock settled”; adapts to slow or fast
  networks without a fixed wait.
- Cons: Thresholds are NIC- and network-specific; new TLVs and failure
  modes; easy to mis-tune; more status surface.

**B — Fixed dwell after `locked → true` (chosen)**

Treat lock as today’s SLAVE (+ GMID as already required). Require that
condition to remain continuously true for `lock_hold_ms` before qualify
returns up. Any unlock clears the timer; the next rising edge starts it
again. This is a fixed delay measured from continuous lock, not a
one-shot sleep that ignores mid-hold unlocks.

- Pros: Simple; uses existing lock signal; one config key; easy to
  test; `0` restores current behavior.
- Cons: May wait longer than needed on a fast lock, or not long enough
  on a pathological servo; does not prove offset is small.

**Choice:** B for simplicity. If distortion still appears after a
generous hold while status stays locked, revisit A as a follow-on.

### Config shape

Add under an existing or small `[ptp]` table (exact key in
implementation; prefer something like `lock_hold_ms`):

- Type: non-negative integer milliseconds.
- Default when absent: `2000` (tunable in implementation if field
  experience suggests otherwise).
- `0`: no dwell (immediate bring-up on lock + GMID).

### Qualify / status

- Track `locked_since: Option<Instant>` in the daemon loop (or equivalent
  pure helper fed by the tick clock in tests).
- Qualify input gains “hold satisfied” (or remaining ms); sources/sinks
  that fail only on hold use a distinct `DownReason` (e.g. `LockHold`).
- Status: PTP line still shows locked/unlocked from `ptp4l`; per-stream
  reason shows the hold. Optionally note remaining hold time.

### Alternatives not chosen for the dwell itself

- One-shot sleep after first `locked == true` without resetting on
  unlock: weaker; rejected in favor of continuous dwell.
- Delaying only the PTP driver create: rejected; the race of interest is
  media to peers after SLAVE, and the driver is still needed for the
  graph clock once streams are allowed up.

## Risks / Trade-offs

- [Default too short] → Distortion remains on slow locks. Mitigation:
  raise default or document a larger `lock_hold_ms` for stubborn NICs.
- [Default too long] → Streams appear late after boot. Mitigation:
  operators set `0` or a smaller value; status shows hold clearly.
- [Hold confused with unlock] → Distinct down reason and status text.
- [Stability still poor after hold] → Fall back to approach A later;
  out of scope here.

## Migration Plan

- Ship daemon with default hold; existing configs without the key pick
  up the default (behavior change: brief post-lock delay).
- Operators who need immediate bring-up set `lock_hold_ms = 0`.
- Rollback: prior daemon binary, or set hold to `0` on a build that
  includes the key.

## Open Questions

- None that block implementation; default `2000` ms may be adjusted
  during apply if bring-up feel on bata suggests otherwise, without
  changing the dwell model.
