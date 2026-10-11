# Proposal

## Why

`uname -r` currently ends with the image tree's git short hash, so every
image commit changes the kernel release string and forces a kernel rebuild
even when the kernel configuration is unchanged. The image git identity
belongs on the OS release, and the kernel local version should name the
board and the kernel configuration revision.

## What Changes

- Append the image tree's short git commit to `/etc/os-release` `VERSION_ID`
  as `-g<shortsha>`, plus `-dirty` when that tree has uncommitted changes.
  `PRETTY_NAME` uses the same suffixed string. `VERSION` and
  `/etc/obake/image-version` stay the release version
  (`image/os/buildroot/VERSION`, or `OBAKE_IMAGE_VERSION` when set), so RAUC
  bundle names and update ordering are unchanged. A tree with no git metadata
  uses `-gnogit`.
- **BREAKING** (identity only): replace the kernel local version
  `-g<shortsha>[-dirty]` with `-<board>-<rev>`. `<board>` is the board id
  (`generic-x86_64`, `generic-aarch64`, `rk35xx`, `qdl`). `<rev>` is the
  1-based index of that board's current kernel-config hash in the ordered set
  of distinct hashes from the image git history of its kernel config inputs.
  The same hash reuses the same rev, including when the config returns to an
  earlier hash. A dirty tree whose hash has never been committed uses the
  next rev for that build and does not write it back into the source tree.
- An image-only commit no longer changes `uname -r` or rebuilds the kernel.
  A kernel source-pin bump still changes the leading version (`7.2.9`) and
  does not by itself bump `<rev>`.
- A shallow image git checkout fails the build, because the history index
  would be too small.
- `rk35xx` gains a board suffix (`-rk35xx-<rev>`). The two generic boards no
  longer share a single `-generic` suffix.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `os-image-build`: `VERSION_ID` carries image git provenance; the kernel
  release string carries board id and config-history rev instead of the image
  git commit. The realtime kernel remains the one built into the image, and
  the image reports that release, but the release is no longer tied to the
  image git revision.
- `image-verification`: guest checks accept the suffixed `VERSION_ID` and the
  `-<board>-<rev>` kernel release, and stop requiring a `-g` segment in
  `uname -r`.

## Impact

- Observable identity: `/etc/os-release` (`VERSION_ID`, `PRETTY_NAME`),
  `uname -r`, and `/lib/modules/<release>`. UKIs embed os-release, so new
  boot entries show the git suffix. `/etc/issue` stays `Welcome to obake-os`.
- OTA: bundle versions, `pinned-inputs.json` `image_version`, and
  `/etc/obake/image-version` stay the release version. Update apply and
  rollback are unchanged. Persistent configuration and shape portability are
  unchanged.
- Build: `image/os/buildroot/build.sh`, `external.mk`, `post-build.sh`,
  board `linux.config` files, and the image git helper. Kernel stamp busting
  follows the config rev, not the image git commit.
- Verification and docs: `image/verification/guest/obake-verify.sh`,
  `image/os/README.md`, and `image/docs/user-guide.md`.
