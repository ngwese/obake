# Tasks

## 1. Config and qualify hold

- [x] 1.1 Add `lock_hold_ms` (default 2000 when absent; `0` allowed) to
      `services/obake-ravenna` config parsing and seed/example TOML
      comments; verify `cargo test -p obake-ravenna` covers absent key,
      zero, and explicit values
- [x] 1.2 Track continuous lock duration and gate qualify on hold
      satisfied; add a distinct down reason for lock hold; verify unit
      tests for hold-not-elapsed, hold-elapsed, and unlock-resets-timer
- [x] 1.3 Surface lock hold in status text distinctly from unlocked;
      verify status formatting tests cover locked+holding vs unlocked

## 2. Docs and headless check

- [x] 2.1 Document `lock_hold_ms` in
      `services/obake-ravenna/REFERENCE_GUIDE.md` (config + qualify/status
      behavior); verify the described default and `0` behavior match the
      implementation
- [x] 2.2 Run `cargo test -p obake-ravenna` headlessly and confirm all
      tests pass
