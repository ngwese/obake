# Proposal

## Why

`--installer` still emits signed RAUC bundles because development signing
material is set for the image keyring, and `post-image` treats that material
as a request to build bundles. Staging then copies every file already in the
Buildroot images directory, so leftover installer images from earlier consoles
are published too. After `make` exits, the cache-to-host copy and the volume
cleanup run with no progress output.

## What Changes

- `--installer` emits installer media and the bootable disk image, and does
  not emit `.raucb` bundles unless bundle generation is explicitly requested.
- Staged installer media is only the variant this build produced, not leftover
  `installer*.img` files from earlier runs.
- `--update` still emits signed bundles and still omits installer media and
  disk images unless those are explicitly re-enabled.
- The build prints progress before it copies artifacts out of the cache volume
  and before it deletes derived images from that volume.
- Development signing material remains available so the installed image trusts
  bundles built later with `--update` on the same host.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `os-image-build`: Installer product does not emit RAUC bundles by default
  and does not republish stale installer images.
- `image-verification`: The build entry point reports progress while staging
  artifacts and cleaning the cache volume, and the installer product selection
  does not include update bundles.

## Impact

- `image/os/buildroot/board/obake/post-image.sh` (bundle gate)
- `image/build/build.sh` (product defaults, staging filter, progress logs)
- `image/docs/user-guide.md` artifact table (`*.raucb` is the update product)
- No change to guest update behavior, OTA, or the persist contract. The
  cache-to-host copy itself stays; only what is copied and what is logged
  changes.
