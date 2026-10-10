# Design

## Context

See proposal.md for motivation. Today:

- Most harnesses default under `image/_verify/<arch>/...`.
- `boot-test.sh` still defaults `BOOT_LOG` to `$here/boot-test.log` beside the
  script. `.gitignore` still exempts `verification/qemu/*.log` and
  `install-work/`. Leftover `nobody`-owned trees from pre-migration runs sit
  there and are outside `ensure-user-owned.sh`'s `_verify` / `_dev` scope.
- Both `image/build/build-env/run.sh` and
  `image/verification/verify-env/run.sh` call `docker build`, which falls back
  to the deprecated legacy builder when Buildx is absent.

## Goals / Non-Goals

**Goals:**

- Put all default harness host-visible outputs under underscore dirs.
- Make reclaim cover known legacy paths so operators can delete them without
  sudo.
- Use Buildx for env image builds; fail clearly when the plugin is missing.
- Document the Buildx host dependency next to existing Docker/KVM notes.

**Non-Goals:**

- Changing guest image behavior, QEMU harness logic, or check suites.
- Installing Buildx for the operator (host package manager only).
- Migrating or reclaiming arbitrary foreign-owned paths outside the known
  legacy harness locations and underscore trees.
- Adopting advanced Buildx features (multi-platform builders, cache exports).

## Decisions

1. **Default `boot-test` log under `_verify`.** Use
   `image/_verify/$OBKA_ARCH/boot/boot-test.log` (create the directory). Keep
   `BOOT_LOG` override. Callers that already pass `BOOT_LOG` (install-test,
   verify-test) are unchanged.
   Alternative: keep writing beside the script — rejected; violates underscore
   output contract and leaves reclaim as a permanent special case.

2. **Reclaim known legacy paths.** Extend `ensure-user-owned.sh` callers (and
   optionally the helper's default list when invoked from verify-env) to include
   `verification/qemu/boot-test.log`, `install-test.log`, and `install-work/`
   when present. Same chown / root one-shot container pattern as today.
   Alternative: document a one-time manual `docker run ... chown` — rejected;
   operators already hit this as a blocker.

3. **Drop gitignore exceptions** for `verification/qemu/*.log` and
   `install-work/` once defaults no longer write there. Leftovers remain local
   and reclaimable; they should not stay as blessed paths.

4. **`docker buildx build` with an explicit preflight.** Before building, both
   wrappers check that `docker buildx version` (or equivalent) succeeds; on
   failure print a short message naming the Ubuntu package `docker-buildx` (and
   the generic "install the Docker Buildx CLI plugin" wording) and exit
   non-zero. Then `docker buildx build -q -t "$image" "$here"`.
   Alternative: keep `docker build` and rely on Docker's default Buildx path —
   rejected; without the plugin it silently falls back to the legacy builder.
   Alternative: soft-fallback with a warning — rejected per product decision
   to fail loudly.

5. **Docs.** Update `image/verification/README.md` and `image/docs/dev-qemu.md`
   (and any build README that lists Docker prerequisites) to require Buildx
   alongside Docker socket / KVM notes.

## Risks / Trade-offs

- **[Risk] Dev hosts without Buildx break until the plugin is installed** →
  Intentional; message points at the package. This host already has
  `docker-buildx` 0.30.1.
- **[Risk] `docker buildx build -q` behavior differs slightly from
  `docker build -q`** → Same tag/context usage; spike once if quiet ID output
  differs.
- **[Risk] Reclaim of paths inside the source tree feels broad** → Limit to the
  three known former defaults; do not walk all of `verification/`.
- **[Trade-off] One-time reclaim vs forcing operators to sudo-delete leftovers**
  → Prefer reclaim so the first verify-env run after this change unblocks them.

## Migration Plan

1. Land Buildx preflight + `buildx build` in both wrappers; update docs.
2. Move `boot-test` default log path; extend reclaim list; remove gitignore
   exceptions.
3. On first verify-env (or explicit `ensure-user-owned.sh`) run, reclaim any
   leftover foreign-owned legacy files so `rm -rf` works without sudo.
4. Rollback is revert of the wrappers/docs; worst case returns to legacy
  builder fallback and source-tree log defaults.
