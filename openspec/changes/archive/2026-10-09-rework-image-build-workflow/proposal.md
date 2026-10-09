# Proposal

## Why

Building and installing an image on real hardware is harder to follow than it
should be. The build entry point lives under `verification/` even though build
and verification are meant to be decoupled; per-board installer choices must be
retyped on every hardware build; the container build leaves root-owned scratch
files in a host output tree that sits outside the image repository; builds for
different boards of the same architecture collide; and the script never says
where its artifacts landed.

## What Changes

- **BREAKING**: move the build entry point and build environment from
  `image/verification/` to a new `image/build/` tree, leaving
  `image/verification/` for QEMU/harness/reproducibility tooling.
- Add per-board build profiles (`default` and `virtual`) that carry the
  installer build defaults (`MAKE_INSTALLER`, `INSTALLER_CONSOLE`,
  `INSTALLER_TARGET_DISK`) so a hardware build does not retype them. An
  unspecified profile means `default`, and an explicit setting still overrides
  the profile.
- **BREAKING**: change the wrapper CLI to `build.sh <board> [profile]`, deriving
  the target architecture from the board instead of taking it separately.
- Emit every host-visible output inside the image repository under
  leading-underscore directories (`_out/`, `_build/`, `_verify/`, `_dev/`) that
  are gitignored.
- Map files written to the host from the build container to the invoking user's
  uid/gid, so outputs are user-owned and removable.
- Give each artifact directory a per-board, per-profile, versioned path
  (`<board>/<profile>/<version>-<shortsha>[-dirty]`) so builds for different
  boards do not collide.
- Print the location of each produced artifact when the build finishes.
- Update the installer and bare-metal documentation and the QEMU harnesses to
  the new entry point, CLI, and output paths.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `os-image-build`: adds per-board build profiles that capture installer build
  defaults and makes the target architecture follow from the board.
- `image-verification`: build tooling now lives outside the verification tree,
  and host build outputs are repository-relative, user-owned, uniquely located,
  and reported.

## Impact

- `image/build/` (new): moved `build.sh`, `build-env/`, and the singularity
  stage builder.
- `image/verification/`: drop the build tools; update `qemu/*.sh`, `qemu/lib.sh`
  and `reproducibility.sh` to the new entry point, CLI, and artifact paths.
- `image/os/buildroot/`: new `profiles/<board>/{default,virtual}.conf` and
  profile/architecture handling in `build.sh`.
- `image/.gitignore`, and the output directories `_build/`, `_out/`, `_verify/`,
  `_dev/`.
- Docs: `image/README.md`, `image/verification/README.md`,
  `image/installer/README.md`, `image/docs/{user-guide,dev-qemu,os-update}.md`.
- No shape, partition-layout, OTA, or persistent-configuration changes.
