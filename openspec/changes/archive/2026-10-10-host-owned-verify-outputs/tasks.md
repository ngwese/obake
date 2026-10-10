# Tasks

## 1. Verify-env identity and groups

- [x] 1.1 Update `image/verification/verify-env/run.sh` to run as
      `--user "$(id -u):$(id -g)"` and, when `/dev/kvm` or
      `/var/run/docker.sock` exists, add `--group-add` for that path's host
      gid. Verify: `grep -E -- '--user|group-add' image/verification/verify-env/run.sh`
      shows both, and a dry read of the script resolves gids only when paths
      exist.
- [x] 1.2 Add a scoped ownership fallback (reuse or mirror
      `ensure_user_owned` from `image/build/build.sh`) so foreign-owned
      `_verify` / `_dev` trees can be reclaimed via a root one-shot container
      chown to the invoker. Verify: invoking the fallback against a
      deliberately foreign-owned directory under `image/_verify` makes
      `rm -rf` succeed without sudo for the invoking user.

## 2. Documentation

- [x] 2.1 Update `image/verification/README.md` to state that verify-env runs
      as the invoking user, that KVM and Docker socket group access are
      required for acceleration / nested builds, and that `_verify` outputs
      are user-owned. Verify: those points appear in the Verification
      environment section.
- [x] 2.2 Update `image/docs/dev-qemu.md` raw `docker run` examples to include
      `--user` and the kvm/docker.sock `--group-add` flags (or point operators
      at `verify-env/run.sh` as the preferred path). Verify: every remaining
      raw `docker run … obake-verify-env` example carries equivalent identity
      flags or is replaced by `run.sh`.

## 3. Headless check

- [x] 3.1 Run a short harness through `image/verification/verify-env/run.sh`
      that writes under `image/_verify` (or reclaim + a minimal write probe),
      then confirm files are owned by the invoker and removable without
      elevated privileges. Verify: `find image/_verify ! -user "$(id -u)"`
      is empty for the paths touched by that run, and `rm -rf` on that subtree
      succeeds without sudo.
