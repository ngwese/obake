# Proposal

## Why

The repository has runtime shape images and a Docker environment for building
the host OS, and no shared compiler image for native work. A single immutable
Singularity definition gives that compiler environment a place in the repo
that later builder definitions can sit beside.

## What Changes

- Add a top-level `builders/` tree, separate from `shapes/` and `image/`.
- Add `builders/builder-base/builder-base.def`, a one-stage definition based
  on Debian trixie-slim.
- The built image is an immutable SIF. Its toolchain is fixed when the
  definition is built: C and C++ (gcc, g++, clang), cmake, make, binutils,
  curl, wget, python3, git, and a rustup-managed stable Rust toolchain for
  the architecture of the machine that built the image.
- The image does not include sudo, and it does not support installing
  additional packages into the SIF after the build.

## Capabilities

### New Capabilities

- `builder-base`: the immutable base compiler image and the tools it provides.

### Modified Capabilities

<!-- none -->

## Impact

- New files under `builders/builder-base/`. No changes to `shapes/`,
  `image/`, host configuration, persistent configuration, or OTA updates.
- Building the image needs network access (Debian packages and the rustup
  installer) and Singularity. The Rust host triple matches the machine that
  runs the build, so an x86_64 build and an aarch64 build are different
  images.
- The stable Rust channel is not pinned. Rebuilding the same definition can
  pick up a newer stable toolchain.
