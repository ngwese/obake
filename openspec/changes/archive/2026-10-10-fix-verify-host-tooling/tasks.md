# Tasks

## 1. Docker Buildx for env wrappers

- [x] 1.1 Add a Buildx preflight and switch
      `image/build/build-env/run.sh` to `docker buildx build`; verify
      `grep -E 'buildx' image/build/build-env/run.sh` shows the preflight and
      build, and that invoking the wrapper with `PATH` hiding buildx prints the
      install hint and exits non-zero
- [x] 1.2 Apply the same Buildx preflight and `docker buildx build` in
      `image/verification/verify-env/run.sh`; verify the same grep and missing-
      buildx failure behavior as 1.1
- [x] 1.3 Document the Buildx host dependency in
      `image/verification/README.md` and `image/docs/dev-qemu.md` (Ubuntu
      package `docker-buildx`); verify those docs name Buildx alongside Docker
      socket / KVM prerequisites

## 2. Harness outputs and reclaim

- [x] 2.1 Default `BOOT_LOG` in `image/verification/qemu/boot-test.sh` to
      `image/_verify/$OBKA_ARCH/boot/boot-test.log` (mkdir as needed); verify
      `grep BOOT_LOG image/verification/qemu/boot-test.sh` no longer defaults
      to `$here/boot-test.log`
- [x] 2.2 Extend `image/verification/verify-env/run.sh` reclaim calls (via
      `ensure-user-owned.sh`) to include known legacy paths
      `verification/qemu/boot-test.log`, `install-test.log`, and
      `install-work/` when present; verify
      `grep -E 'boot-test.log|install-work|install-test.log'
      image/verification/verify-env/run.sh` (or the helper) lists them
- [x] 2.3 Remove `verification/qemu/*.log` and
      `verification/qemu/install-work/` exceptions from `image/.gitignore`;
      verify those lines are gone
- [x] 2.4 Document reclaim of legacy `verification/qemu` scratch in
      `image/verification/README.md`; verify the README mentions those paths
      (or `ensure-user-owned.sh`) for leftover cleanup

## 3. Headless smoke

- [x] 3.1 With Buildx installed, run
      `image/verification/ensure-user-owned.sh` against `_verify`, `_dev`, and
      the legacy qemu paths (if present), then confirm any previously
      foreign-owned leftover under `verification/qemu/` is owned by the
      invoking user and removable without elevated privileges
- [x] 3.2 Invoke `image/build/build-env/run.sh true` (or equivalent no-op)
      and confirm the env image builds via Buildx with no legacy-builder
      deprecation notice on stderr
