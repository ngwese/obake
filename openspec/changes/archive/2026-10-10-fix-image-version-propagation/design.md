# Design

## Context

See proposal.md for motivation. Today version resolution is inconsistent:

| Consumer | Resolution |
|----------|------------|
| `post-build.sh` | `OBAKE_IMAGE_VERSION` → `buildroot/VERSION` → `0.0.0` |
| `post-image.sh` (bundles, `VERSION`) | same |
| `pinned-inputs.sh` | `OBAKE_IMAGE_VERSION` → `0.0.0` only |
| `image/build/build.sh` artifact dir | reads `buildroot/VERSION` |

`post-image.sh` only `rm`s `obake-$version-$slot.raucb` before rebuilding that
slot; prior-version `obake-*.raucb` remain in the persistent Buildroot images
tree (Docker volume). Staging copies every top-level file into `_out/…`, so
stale bundles appear next to the new ones. Cache cleanup already drops
`disk-*.img` and scratch dirs, not RAUC bundles.

## Goals / Non-Goals

**Goals:**
- One shared resolution path so `pinned-inputs.json` matches os-release,
  `/etc/obake/image-version`, bundles, and the `VERSION` stamp
- Incremental rebuilds after a version bump do not stage prior-version
  `obake-*.raucb`
- Headless check that catches a future regression without a QEMU boot

**Non-Goals:**
- Cleaning stale installer images from prior console modes (separate issue;
  installers are not versioned in the filename the same way)
- Changing how verification harnesses override versions via
  `OBAKE_IMAGE_VERSION`
- Reworking artifact directory naming or RAUC signing material layout
- Auto-deleting old `_out/os/<board>/<profile>/<old-version>-*` directories

## Decisions

### 1. Resolve version inside `pinned-inputs.sh`

Give `pinned-inputs.sh` the same chain as post-build, preferring the baked
rootfs when `TARGET_DIR` is passed:

1. `OBAKE_IMAGE_VERSION` if set and non-empty
2. `$target/etc/obake/image-version` when the target path is provided
3. `$brroot/VERSION` (`image/os/buildroot/VERSION`)
4. `0.0.0`

Preferring the baked file ties the JSON to what post-build already wrote for
that rootfs. Harnesses that set `OBAKE_IMAGE_VERSION` still win.

Alternative considered: pass `$version` from `post-image.sh` as a fifth
argument. Rejected as the only fix — callers other than post-image would keep
the bug. Passing an explicit override remains optional later if needed.

### 2. Delete non-current `obake-*.raucb` before emitting bundles

In `post-image.sh`, when RAUC signing is configured, remove every
`obake-*.raucb` in `$binaries` that is not `obake-$version-*.raucb` (then
rebuild current slots as today). That cleans the Buildroot images tree before
staging, so Docker and non-Docker paths both benefit.

Also extend `image/build/build.sh` Docker post-build cleanup to drop
`obake-*.raucb` from the cache images dir after staging (mirrors
`disk-*.img`), so a failed mid-build cannot leave orphans for the next stage
copy. Prefer the post-image cleanup as the primary guarantee; cache cleanup
is defense in depth.

Alternative considered: filter during staging (copy only current-version
bundles). That would hide leftovers in the cache without fixing the images
tree; post-image cleanup is clearer.

### 3. Host-side artifact check, not a new guest test

Add a small assertion where artifact directories are already consumed (for
example `image/verification/qemu/lib.sh` after a build, or a tiny helper
invoked from an existing smoke path) that:

```
jq -r .image_version pinned-inputs.json == $(cat VERSION)
```

No guest boot required. Guest branding checks already cover
`/etc/obake/image-version` vs os-release.

### 4. Refresh reproducibility record when regenerating

`image/verification/reproducibility/<board>.pinned-inputs.json` is rewritten
by `reproducibility.sh`. After this fix, a regenerated record will carry the
tree's `VERSION`. No special migration; update the checked-in record only if
the project chooses to re-run and commit a reproducibility refresh.

## Risks / Trade-offs

- [Risk] Preferring `/etc/obake/image-version` could diverge if post-build
  and pinned-inputs disagree on resolution → Mitigation: same fallback order;
  `OBAKE_IMAGE_VERSION` still overrides both when set.
- [Risk] Aggressive `rm obake-*.raucb` deletes bundles an operator left in
  the Buildroot images dir by hand → Mitigation: that directory is build
  scratch; durable copies live under `_out/…/<version>-<sha>/`.
- [Risk] Verification harnesses that build multiple versions into one
  `OBAKE_OUTDIR` (integration test uses separate out dirs today) could lose
  sibling bundles → Mitigation: confirm harnesses use distinct outdirs;
  cleanup is scoped to the active `$binaries` images dir for one build.

## Migration Plan

- Land script fixes; no host OTA migration.
- Operators with polluted `_out` or Docker cache can delete stale
  `obake-<old>-*.raucb` once; subsequent builds self-clean.
- Optional: regenerate reproducibility pinned-inputs records after the fix.

## Open Questions

None.
