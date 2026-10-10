# Proposal

## Why

Singularity CE is shipped `--without-suid` and mounts SIF images over FUSE, which
requires user namespaces, but the host kernel never enabled them. On hardware,
`singularity run` fails with `user namespace not supported by your system`, so
shapes cannot run. Guest verification only checked the binary version as root,
so the gap never failed headless CI.

## What Changes

- Enable kernel namespaces (including `USER_NS`) and the usual cgroup controllers
  on every board via the shared host kernel fragment; keep Singularity
  `--without-suid` (no setuid helper).
- Make guest verification run as the `obake` login user (strict), using only the
  same privilege paths operators have (`sudo` for `obake-update`, polkit for
  systemd/power).
- Exercise Singularity by running a tiny local SIF as `obake` in guest checks
  (not a network `library://` pull).
- Route RAUC install/reject/rollback guest checks through `obake-update`, with
  an optional local bundle path on `apply`.
- Align contract probes with SSH reality: `obake` can write under `/home`;
  writes to `/etc` and the read-only root fail. Observe `/etc` overlay topology
  without requiring the login user to write `/etc`.
- **BREAKING** (kernel only): newly built images enable user namespaces and
  additional cgroup controllers. Persist, update apply paths, and login
  credentials are unchanged; hosts pick this up on the next OTA slot.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `os-image-build`: require kernel namespace and cgroup support sufficient for
  the non-setuid Singularity runtime; require that the shipped runtime can run
  a container as the host login user.
- `image-verification`: require guest functional checks (and related guest
  suites) to run as `obake`; require a Singularity run smoke test; require
  update checks via `obake-update`; require contract checks that match the
  login-user writable surface.
- `host-config`: clarify that the `obake` SSH user may write under `/home` and
  SHALL NOT write `/etc` or the read-only root (directory-contract mounts remain
  system-writable as today).
- `os-update`: allow `obake-update apply` to install a caller-supplied local
  bundle path (in addition to USB / HTTPS / configured `local_bundle`).

## Impact

- `image/os/buildroot/board/obake/linux-obake.fragment` (and any board config
  cleanup needed so the shared fragment wins).
- Guest verify services and scripts under `image/verification/guest/` and
  `image/os/buildroot/board/obake/install-verify.sh`.
- `obake-update` CLI and docs (`image/docs/os-update.md`, host-config /
  user-guide notes on Singularity and the login-user contract).
- QEMU verify harnesses that assert guest check names or ordering.
- OTA: kernel change ships in a new signed image; no persist migration.
- Shapes unchanged; they become runnable on updated hosts.
