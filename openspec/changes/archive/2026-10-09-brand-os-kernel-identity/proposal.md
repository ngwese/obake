# Proposal

## Why

Built images still present as Buildroot at login and in `/etc/os-release`, and
the kernel release string does not identify which board or which revision of
the image tree produced it. Operators and support need a clear OS brand and a
`uname -r` that ties the running kernel to board and git revision.

## What Changes

- Brand the image as **obake-os** in `/etc/os-release` (`NAME`, `ID`,
  `VERSION`, `VERSION_ID`, `PRETTY_NAME`), using
  `image/os/buildroot/VERSION` for `VERSION` and `VERSION_ID`.
- Set the login banner (`/etc/issue`) to `Welcome to obake-os` (today's
  "Welcome to Buildroot" comes from Buildroot's issue string, not os-release).
- Give every board's kernel a release suffix `-g<shortsha>[-dirty]` derived
  from the image git tree (same provenance pattern as artifact directories).
- Give board-owned LOCALVERSION suffixes: `-qdl` for `qdl`, `-generic` for
  both generic boards; leave `rk35xx` without a board suffix for now.
- **BREAKING** (identity only): `uname -r`, `/etc/os-release`, and the login
  banner change on newly built images. Update apply and persist paths are
  unchanged; existing hosts keep their prior identity until updated.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `os-image-build`: require obake-os branding in os-release and issue; require
  kernel release strings that include board LOCALVERSION (where defined) and
  image-tree `-g<shortsha>[-dirty]` provenance.
- `image-verification`: assert OS branding and kernel release shape in
  headless guest checks where practical.

## Impact

- Buildroot base/board configs, kernel configs or generated localversion
  fragments, and post-build (or post-fakeroot) hooks under
  `image/os/buildroot/`.
- UKIs embed os-release via `ukify --os-release`; new images carry obake-os
  identity in the UKI as well as the rootfs.
- `/etc/obake/image-version` remains the VERSION file contents; os-release
  aligns with it. Artifact directory naming
  (`<version>-<shortsha>[-dirty]`) is unchanged.
- Embedding git shortsha/`-dirty` in the kernel release means rootfs content
  changes with revision (and dirty trees); pin-level input selection stays
  deterministic. OTA and persist contracts are unaffected.
- Docs that mention Buildroot as the user-visible OS name should be updated
  where they describe login or os-release.
