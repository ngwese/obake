# Proposal

## Why

Bumping `image/os/buildroot/VERSION` does not propagate through every build
artifact: `pinned-inputs.json` (and the copy embedded in
`image-manifest.json`) still records `image_version: 0.0.0` unless
`OBAKE_IMAGE_VERSION` is set explicitly. Separately, incremental Buildroot
builds leave prior-version RAUC bundles in the images tree, so staging copies
stale `obake-<old>-*.raucb` into the new artifact directory. Operators need a
single version bump to keep identity, manifests, bundles, and staged outputs
aligned.

## What Changes

- Resolve the image version for `pinned-inputs.json` with the same rules used
  by post-build / post-image: `OBAKE_IMAGE_VERSION`, else the built rootfs
  `/etc/obake/image-version` when available, else `buildroot/VERSION`, else
  `0.0.0`.
- Ensure a completed build's `pinned-inputs.json` `image_version` matches the
  version recorded in `VERSION`, bundles, and `/etc/obake/image-version`.
- Remove prior-version `obake-*.raucb` from the Buildroot images directory
  when emitting bundles for the current version, so staging does not publish
  stale update bundles alongside the new ones.
- Add a headless verification check (or extend an existing one) that
  `pinned-inputs.json` `image_version` equals the artifact `VERSION` stamp.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `os-image-build`: require pinned-inputs `image_version` to follow the same
  image-version resolution as os-release / bundles; require builds not to
  leave prior-version RAUC bundles in the staged artifact set.
- `image-verification`: assert `pinned-inputs.json` `image_version` matches
  the artifact `VERSION` after a build.

## Impact

- `image/os/scripts/pinned-inputs.sh` and its call site in
  `image/os/buildroot/board/obake/post-image.sh`.
- Bundle emission / cleanup in post-image (and possibly the Docker staging
  cleanup in `image/build/build.sh`).
- Verification under `image/verification/` that can inspect a staged artifact
  directory without a full guest boot when practical.
- No change to OTA apply semantics, persist, or host contracts. Existing hosts
  are unaffected until rebuilt. Reproducibility golden
  `*.pinned-inputs.json` may need a refresh if they hardcode `0.0.0` while
  the tree's `VERSION` has moved.
