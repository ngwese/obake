# Spec Delta

## ADDED Requirements

### Requirement: Pinned inputs record the image version

The build SHALL record `image_version` in `pinned-inputs.json` using the same
image version that brands the rootfs and names update bundles:
`OBAKE_IMAGE_VERSION` when set, otherwise the version baked into the image
(or `image/os/buildroot/VERSION` when that is the source), defaulting to
`0.0.0` only when no version source is available.

#### Scenario: VERSION file propagates without override

- **WHEN** an image is built with `OBAKE_IMAGE_VERSION` unset and
  `image/os/buildroot/VERSION` containing a non-default version
- **THEN** `pinned-inputs.json` `image_version` equals that VERSION file
  contents and equals the artifact `VERSION` stamp

#### Scenario: Explicit override wins

- **WHEN** an image is built with `OBAKE_IMAGE_VERSION` set
- **THEN** `pinned-inputs.json` `image_version` equals
  `OBAKE_IMAGE_VERSION` and matches `/etc/obake/image-version` in the
  rootfs

### Requirement: Staged artifacts omit prior-version RAUC bundles

When a build emits signed RAUC update bundles for the current image version,
the Buildroot images tree and the staged artifact directory SHALL NOT retain
`obake-<other-version>-*.raucb` files from earlier builds of the same board
output.

#### Scenario: Version bump drops old bundles

- **WHEN** a board is rebuilt after the image version changes and RAUC signing
  material is configured
- **THEN** the staged artifact directory contains bundles named for the
  current version only and does not contain bundles named for the previous
  version
