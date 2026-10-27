# Tasks

## 1. On-device apply flags

- [x] 1.1 Extend
  `image/os/buildroot/board/obake/rootfs-overlay/usr/bin/obake-update`
  so `apply` accepts `--cleanup` and `--reboot` (with optional bundle
  path), deletes an explicit path only after successful install when
  `--cleanup` is set, and runs `systemctl reboot` after cleanup when
  `--reboot` is set; verify by running the script under a temp root or
  dry inspection that failed install skips both actions and usage text
  lists the new flags
- [x] 1.2 Update `image/docs/os-update.md` and the user-guide update
  section for `--cleanup` / `--reboot`; verify the documented
  `obake-update apply --cleanup --reboot <path>` example matches the
  script usage

## 2. CLI bundle selection

- [x] 2.1 Add filename parse and semver compare helpers in
  `services/obake` (pattern `obake-<MAJOR.MINOR.PATCH>-<a|b>.raucb`)
  with unit tests for parse failures, ordering, and `>=` eligibility
- [x] 2.2 Implement directory selection (inactive slot + latest eligible
  semver) and file validation (`--force` bypass; `--force` with a
  directory is an error) with unit tests covering the spec scenarios
  (newest eligible, nothing new enough, same version, wrong slot,
  older file, force)

## 3. CLI device update over SSH

- [x] 3.1 Add `obake device update --target user@host PATH [--force]`
  in `services/obake` (new module + `main.rs` wiring as a non-GraphQL
  noun like `auth`); verify `obake device update --help` shows the
  required `--target` and that omitting it fails before SSH
- [x] 3.2 Implement SSH probe of `obake-update status`, mapping active
  slot to inactive `a`/`b`, SCP to `/home/.obake-update/<basename>`,
  and remote `obake-update apply --cleanup --reboot <staged>`; verify
  with a PATH-stubbed ssh/scp test or scripted fake that the remote
  command line includes both flags and the staged path
- [x] 3.3 Document `device update` in `services/obake/README.md` as a
  non-schema noun beside `auth`; verify the README command table lists
  transport as SSH

## 4. Headless verification

- [x] 4.1 Run `cargo test -p obake` (workspace lockfile) and confirm
  new selection/SSH-stub tests pass without PipeWire, JACK, or
  Singularity
