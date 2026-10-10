# Tasks

## 1. Kernel namespaces and cgroups

- [x] 1.1 Add namespaces and common cgroup controllers to
      `image/os/buildroot/board/obake/linux-obake.fragment` per design.md
      (USER_NS and companions; memcg/cpuset/device/cpuacct/pids/freezer;
      no `RT_GROUP_SCHED`). Verify: fragment contains the listed options and
      does not enable `CONFIG_RT_GROUP_SCHED`.
- [x] 1.2 Scan board `linux.config` files under
      `image/os/buildroot/board/obake/{qdl,generic-x86_64,generic-aarch64}/`
      for explicit disables that would override the fragment; remove or
      correct conflicts. Verify: no board config sets those options to `n`
      against the fragment.

## 2. Update client local bundle path

- [x] 2.1 Extend `image/os/buildroot/board/obake/rootfs-overlay/usr/bin/obake-update`
      so `apply [<bundle-path>]` installs a caller-supplied `.raucb` with the
      same signature path as other sources; keep USB/HTTPS/`local_bundle=`
      when no path is given. Verify: usage text and apply branch handle a
      path; missing path preserves prior discovery order.
- [x] 2.2 Document the local path form in `image/docs/os-update.md`. Verify:
      docs show `obake-update apply /path/to/bundle.raucb` alongside USB and
      HTTPS.

## 3. Guest verification as obake

- [x] 3.1 Update `image/os/buildroot/board/obake/install-verify.sh` so guest
      oneshot services (`obake-verify` and update/persist/health/integration
      suites) run as `User=obake` / `Group=obake` and power off via a
      polkit-allowed path. Verify: generated unit files set `User=obake` and
      no longer call `systemctl poweroff -f` as root-only.
- [x] 3.2 Rewrite `image/verification/guest/obake-verify.sh` contract, root-
      lock, and RAUC probes for login-user reality (home writable; `/etc` and
      root writes fail; overlay via `findmnt`; empty-password `su` fails;
      install/reject/rollback via `obake-update`). Verify: script contains no
      raw `rauc install`, no `/etc/shadow` read, and no `/etc` write probe.
- [x] 3.3 Rewrite `image/verification/guest/obake-update-test.sh`,
      `obake-persist-test.sh`, and related guest suites to run without root-
      only `rauc`/`/etc` writes; use `obake-update` and `/home` markers.
      Verify: guest scripts do not call `rauc install` or write `/etc` as the
      check process.
- [x] 3.4 Add a tiny local SIF fixture under `image/verification/` (or build
      step) and install it at `/opt/obake/verify.sif` when `VERIFY=1`; add a
      guest check that `singularity run` of that SIF succeeds as `obake`.
      Verify: fixture path exists in verify install path; `obake-verify.sh`
      asserts a successful local run (not `library://`).

## 4. Host-config docs

- [x] 4.1 Update `image/docs/host-config.md` (and brief user-guide /
      dev-qemu notes as needed) so the directory contract matches SSH
      reality: `obake` writes `/home`, cannot write `/etc` or the read-only
      root; note Singularity needs user namespaces. Verify: docs no longer
      imply `obake` can write `/etc`, and mention a local `singularity run`
      smoke path.

## 5. Headless verification

- [x] 5.1 Run `image/verification/qemu/verify-test.sh` (rebuild as needed)
      and confirm guest checks pass as `obake`, including the Singularity
      local SIF run and `obake-update`-based install/reject paths. Verify:
      harness reports `OBKA_VERIFY_SUITE_PASS` (or equivalent) with the new
      check names succeeding.
