# Tasks

## 1. Seed ptp4l step_threshold (image)

- [x] 1.1 Add `step_threshold 0.001` to
  `image/os/buildroot/board/obake/qdl/rootfs-overlay/usr/share/obake/linuxptp/ptp4l.cfg`
  and verify the seeded file contains a non-zero `step_threshold` while
  `phc2sys.env` still slews only
- [x] 1.2 Document the key and existing-host migration in
  `image/docs/user-guide.md` (linuxptp section) and verify the guide
  tells operators to add `step_threshold` and restart `ptp4l` when the
  config predates the seed

## 2. PTP offset over management UDS (obake-ravenna)

- [x] 2.1 Extend `services/obake-ravenna/src/ptp.rs` to GET
  `TIME_STATUS_NP` and parse `master_offset` into `PtpState` (or an
  adjacent field); on parse/GET failure leave offset unset and verify
  unit tests cover a fixture response and a failure path
- [x] 2.2 Wire the offset into the daemon tick / qualify input and
  verify a debug log shows the offset on each successful query

## 3. Config, qualify, and status (obake-ravenna)

- [x] 3.1 Add `[ptp].max_offset_ns` to `config.rs` (default
  `1000000`, `0` disables) and verify config unit tests for absent,
  zero, and explicit values
- [x] 3.2 Add `DownReason::OffsetHigh` (`offset-high`), gate sources
  and sinks when `|offset| > max_offset_ns` (and when offset is missing
  while the threshold is non-zero), and verify qualify unit tests for
  high, within-threshold, disabled (`0`), and missing-offset cases
- [x] 3.3 Surface `offset-high` in status / README down-reason table and
  verify `obake-ravenna status` text distinguishes it from `unlocked`
  and `lock-hold` in status unit tests

## 4. Docs and headless check

- [x] 4.1 Document `[ptp].max_offset_ns` in
  `services/obake-ravenna/REFERENCE_GUIDE.md` and
  `services/obake-ravenna/examples/config.toml` and verify the guide
  states default, zero-disable, and interaction with lock-hold
- [x] 4.2 Run `cargo test` in `services/obake-ravenna` headlessly and
  verify all tests pass
