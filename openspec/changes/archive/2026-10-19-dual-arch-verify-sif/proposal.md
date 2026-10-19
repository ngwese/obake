# Proposal

## Why

Guest functional checks embed a single `verify.sif` that is currently amd64.
On an aarch64 VERIFY image, Singularity rejects it (`architecture (amd64)
could not run on the host's (arm64)`), so singularity run/fakeroot/build
checks fail even when the host Singularity stack is fine. Verification needs
a fixture that matches the guest architecture.

## What Changes

- Ship both `verify-x86_64.sif` and `verify-arm64.sif` under
  `image/verification/fixtures/` (git-lfs), built from the existing
  `verify.def`.
- **BREAKING**: drop the single `verify.sif` fixture name; regenerate docs
  and install logic use the arch-specific names.
- When `VERIFY=1` embeds the fixture, select the SIF for the image's target
  architecture (`x86_64` → `verify-x86_64.sif`, `aarch64` →
  `verify-arm64.sif`) and still install it as `/opt/obake/verify.sif` so
  guest scripts stay unchanged.
- Document how to rebuild both fixtures.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `image-verification`: VERIFY images SHALL embed a Singularity fixture whose
  architecture matches the guest, chosen from arch-specific fixture files.

## Impact

- `image/verification/fixtures/` — add `verify-x86_64.sif` and
  `verify-arm64.sif`; remove `verify.sif`; update README.
- `image/os/buildroot/board/obake/install-verify.sh` — pick the fixture by
  target arch (from Buildroot config / post-build arch).
- `image/.gitattributes` — already globs `verification/fixtures/*.sif`; no
  pattern change expected.
- Guest scripts under `image/verification/guest/` keep using
  `/opt/obake/verify.sif`.
- No impact on production (non-VERIFY) images, OTA, or shape portability.
