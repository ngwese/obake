# Proposal

## Why

Verification still leaves host-visible scratch next to harness scripts
(`verification/qemu/*.log`, leftover `install-work/`) that can land as
foreign-owned (`nobody` under mapped Docker) and cannot be removed without
elevated privileges, even though underscore trees under `image/_verify` are
already covered. Separately, building the build-env and verify-env images with
plain `docker build` falls back to the deprecated legacy builder when Buildx
is missing, printing a deprecation warning on every run.

## What Changes

- Default `boot-test` (and any remaining harness) host-visible logs/work into
  underscore output directories under `image/_verify/`, matching other
  harnesses.
- Widen the ownership reclaim helper (and verify-env's use of it) so known
  legacy scratch under `verification/qemu/` can be reclaimed without sudo,
  then drop gitignore exceptions once nothing writes there.
- Switch `build-env/run.sh` and `verify-env/run.sh` to `docker buildx build`.
- **BREAKING** (dev hosts only): if the Buildx CLI plugin is missing, those
  wrappers fail immediately with a short install hint instead of falling back
  to the legacy builder.
- Document Buildx as a host prerequisite for containerized build and
  verification.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `image-verification`: Require verification harness host-visible outputs under
  underscore dirs; reclaim known legacy foreign-owned scratch without elevated
  privileges; require Buildx for env image builds with a loud failure and
  documented host dependency.

## Impact

- `image/build/build-env/run.sh`, `image/verification/verify-env/run.sh`
- `image/verification/qemu/boot-test.sh` (default log path)
- `image/verification/ensure-user-owned.sh` and callers
- `image/.gitignore` (remove obsolete `verification/qemu` scratch exceptions)
- Operator docs: `image/verification/README.md`, possibly `image/docs/dev-qemu.md`
  and build docs that mention Docker
- Dev hosts need the Buildx plugin (`docker-buildx` on Ubuntu); no guest image,
  OTA, or persist-contract impact
