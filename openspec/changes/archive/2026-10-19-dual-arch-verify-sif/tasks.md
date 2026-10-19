# Tasks

## 1. Fixtures

- [x] 1.1 Build `image/verification/fixtures/verify-x86_64.sif` and
  `image/verification/fixtures/verify-arm64.sif` from `verify.def`, track
  both with git-lfs, and verify each SIF reports the expected architecture
  (e.g. via `singularity sif list` / inspect)
- [x] 1.2 Remove `image/verification/fixtures/verify.sif` and update
  `image/verification/fixtures/README.md` with regenerate commands for both
  arches, and verify the README names `verify-x86_64.sif` and
  `verify-arm64.sif` with no remaining `verify.sif` install path

## 2. Embed by guest architecture

- [x] 2.1 Update `embed_verify_sif` in
  `image/os/buildroot/board/obake/install-verify.sh` to select
  `verify-x86_64.sif` or `verify-arm64.sif` from the target arch
  (`BR2_x86_64` / `BR2_aarch64`, matching post-build), install as
  `/opt/obake/verify.sif`, and fail if the fixture or arch is missing;
  verify the script contains both fixture names and the arch mapping
- [x] 2.2 Confirm guest scripts still reference only `/opt/obake/verify.sif`
  (no path changes in `image/verification/guest/obake-verify.sh`)

## 3. Headless verification

- [x] 3.1 Rebuild a `VERIFY=1` `generic-aarch64` virtual image and run the
  QEMU guest functional check, and verify
  `OBKA_CHECK singularity-run=pass`,
  `OBKA_CHECK singularity-fakeroot=pass`, and
  `OBKA_CHECK tmp-singularity-build=pass` (or the suite reaches
  `OBKA_VERIFY_PASS` when other checks are green)
