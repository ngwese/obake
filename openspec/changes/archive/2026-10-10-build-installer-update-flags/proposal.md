# Proposal

## Why

Iterating on an established board via RAUC (`scp` + `obake-update apply`)
requires remembering signing-key environment variables and overriding profile
installer defaults. The build CLI should make the two real developer products —
first-flash installer media and update bundles — explicit and short.

## What Changes

- **BREAKING**: `image/build/build.sh <board> [profile]` requires exactly one of
  `--installer` or `--update`. Omitting both or passing both fails with usage.
  `--print-dir` stays flag-free (path discovery only).
- `--installer` forces installer emission and applies default development
  signing/keyring material when unset so a board flashed from that host accepts
  later `--update` bundles from the same material.
- `--update` disables installer and full-disk assembly (`MAKE_INSTALLER=0`,
  `MAKE_DISK=0`), ensures development signing material under
  `image/_build/signing` (create if missing), and emits signed
  `obake-<version>-{a,b}.raucb` artifacts.
- Explicit `RAUC_*` / `MAKE_*` environment variables still override defaults.
- Update all in-tree callers (verification harnesses, docs) to pass the required
  product flag.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `image-verification`: Build entry point requires a product flag; document the
  installer vs update workflows and default signing material.
- `os-image-build`: Board profiles remain; product intent (`--installer` /
  `--update`) selects which optional post-image artifacts are produced and how
  development signing defaults apply.

## Impact

- `image/build/build.sh` CLI and signing defaults
- `image/os/buildroot` post-image knobs consumed by those flags
- Verification harnesses under `image/verification/qemu/`
- Docs: `image/docs/{user-guide,os-update,dev-qemu}.md`,
  `image/verification/README.md`, `image/installer/README.md`, `image/os/README.md`
- No change to on-device update semantics once a bundle is signed and trusted
