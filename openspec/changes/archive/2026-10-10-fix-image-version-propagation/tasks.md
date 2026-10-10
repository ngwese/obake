# Tasks

## 1. Version resolution in pinned-inputs

- [x] 1.1 Update `image/os/scripts/pinned-inputs.sh` to resolve
      `image_version` as: `OBAKE_IMAGE_VERSION` if set; else
      `$target/etc/obake/image-version` when a target dir is passed; else
      `image/os/buildroot/VERSION`; else `0.0.0`. Verify: invoking the
      script with `OBAKE_IMAGE_VERSION` unset and a target that contains
      `etc/obake/image-version=0.1.0` writes `"image_version": "0.1.0"`.
- [x] 1.2 Confirm `post-image.sh` still passes `$TARGET_DIR` into
      `pinned-inputs.sh` so the baked image-version is preferred. Verify:
      call site includes the target directory argument.

## 2. Stale RAUC bundle cleanup

- [x] 2.1 In `image/os/buildroot/board/obake/post-image.sh`, before
      emitting current-version bundles, remove any
      `obake-*.raucb` in `$binaries` that do not match
      `obake-$version-*.raucb`. Verify: with a fake prior-version
      `obake-0.0.0-a.raucb` present, a post-image run with version
      `0.1.0` leaves only `obake-0.1.0-*.raucb`.
- [x] 2.2 Extend Docker post-stage cleanup in `image/build/build.sh` to
      also remove `obake-*.raucb` from the cached images directory (same
      pattern as `disk-*.img`). Verify: the cleanup command lists
      `obake-*.raucb`.

## 3. Verification and docs

- [x] 3.1 Add a headless check (helper and/or hook in
      `image/verification/qemu/lib.sh` after staging) that fails when
      `pinned-inputs.json` `image_version` differs from artifact
      `VERSION`. Verify: matching stamps pass; a deliberately mismatched
      JSON fails.
- [x] 3.2 Note in `image/os/README.md` (Outputs / version section) that
      `pinned-inputs.json` `image_version` follows the same resolution as
      `/etc/obake/image-version`, and that prior-version RAUC bundles are
      removed from the images tree. Verify: docs mention both behaviors.
- [x] 3.3 Rebuild a board with RAUC signing (e.g. `qdl` default) and
      confirm staged artifacts: `VERSION`, `pinned-inputs.json`
      `image_version`, and `obake-<version>-*.raucb` agree, with no
      prior-version `obake-*.raucb`. Verify: headless check from 3.1
      passes on the staged directory.
