# Design

## Context

See proposal.md for motivation. Today `image/build/build.sh <board> [profile]`
applies `profiles/<board>/{default,virtual}.conf` (`MAKE_INSTALLER`, `CONSOLE`,
`INSTALLER_TARGET_DISK`). Signed bundles appear only when `RAUC_SIGNING_CERT`
and `RAUC_SIGNING_KEY` are set (`post-image.sh`). `qdl` `default` forces
`MAKE_INSTALLER=1`, so the fast RAUC loop needs manual overrides. Development
signing already has `image/os/scripts/make-signing-material.sh` (defaulting
historically under `.build/signing`; docs point at `image/_build/signing`).

## Goals / Non-Goals

**Goals:**

- Make installer vs update an explicit, required CLI choice.
- Default signing/keyring so same-host install→update just works.
- Keep `default`/`virtual` as target shape (physical vs verify), not products.
- Update every in-tree caller and the developer docs.

**Non-Goals:**

- Changing RAUC on-device apply/rollback semantics.
- Shipping production keys in-tree.
- A third product that emits “everything” in one flag (operators can override
  env after choosing a product if needed).

## Decisions

1. **Required XOR flags on `image/build/build.sh`.** Parse
   `--installer` / `--update` alongside the optional profile token. Reject
   missing/both before starting the container. Alternative: optional flags with
   profile defaults — rejected; user asked to require the flag for now.

2. **`--print-dir` stays product-free.** It only computes the artifact path
   from board/profile/version/sha; no build side effects.

3. **Flag semantics as env defaults before profile/post-image.**
   - `--installer`: export `MAKE_INSTALLER=1` (force); ensure default signing
     dir; if `RAUC_KEYRING` / `RAUC_SIGNING_*` unset, point them at
     `_build/signing` (create material if needed). Still allow profile
     `CONSOLE` / `INSTALLER_TARGET_DISK`. Disk image stays at today's default
     (`MAKE_DISK` unset → 1) so installer builds keep a bootable disk unless
     overridden.
   - `--update`: export `MAKE_INSTALLER=0` and `MAKE_DISK=0` unless already
     set in the environment; ensure signing material; set unset `RAUC_*` to
     the default paths so `post-image.sh` emits both slot bundles.

4. **Default signing path `image/_build/signing`.** Call
   `make-signing-material.sh` when cert/key/CA are missing. Align that
   script's default outdir with `_build/signing` if it still points at
   `.build/signing`. Explicit env always wins. Alternative: commit a shared
   dev CA — rejected; generating per clone is enough for same-host loops and
   avoids publishing a private key.

5. **Lower-level `image/os/buildroot/build.sh` unchanged CLI.** Product flags
   live on the public entry point (`image/build/build.sh`). Harnesses and
   docs standardize on that wrapper. Direct Buildroot builds remain an escape
   hatch with raw env knobs.

6. **Caller migration.** Every `image/build/build.sh …` invocation in
   verification harnesses and docs gains `--installer` or `--update`:
   - Install / reinstall / interactive / dev-vm / ssh disk images →
     `--installer` (or `--update` only where the harness truly builds bundles
     alone, e.g. update/health bundle stages).
   - Bundle-only stages that already pass signing env → `--update` (signing
     env remains an override).

## Risks / Trade-offs

- **[Risk] Breaking every bare `build.sh <board>` invocation** → Mitigate by
  updating in-tree callers in the same change; usage error text names the
  flags.
- **[Risk] Board flashed with a different CA rejects new bundles** → Document
  that the loop assumes one shared `_build/signing` for that fleet; production
  keys stay explicit overrides.
- **[Risk] `--installer` always regenerating signing changes the baked
  keyring across cleans of `_build`** → Accept for dev; operators who need
  stability keep the directory or set `RAUC_*` explicitly.
- **[Trade-off] No combined `--installer --update`** → Keeps the XOR mental
  model; rare “build both” can set env overrides on one product later if
  needed.

## Migration Plan

1. Implement flag parsing and signing defaults in `image/build/build.sh`.
2. Update harnesses and docs in the same change.
3. Operators: replace `build.sh qdl` with `build.sh qdl --installer` or
   `build.sh qdl --update`.
