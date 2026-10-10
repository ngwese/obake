# Spec Delta

## ADDED Requirements

### Requirement: obake-os release identity

The built image SHALL present as `obake-os` in `/etc/os-release`: `NAME` and
`ID` SHALL be `obake-os`; `VERSION` and `VERSION_ID` SHALL equal the contents
of `image/os/buildroot/VERSION` (or `OBAKE_IMAGE_VERSION` when set); and
`PRETTY_NAME` SHALL be `obake-os` followed by that version string.

#### Scenario: os-release fields

- **WHEN** an image is built
- **THEN** `/etc/os-release` has `NAME=obake-os`, `ID=obake-os`,
  `VERSION` and `VERSION_ID` matching the image version, and
  `PRETTY_NAME="obake-os <version>"`

#### Scenario: Aligns with image-version

- **WHEN** an image is built
- **THEN** os-release `VERSION` equals the contents of
  `/etc/obake/image-version`

### Requirement: obake-os login banner

The built image SHALL display `Welcome to obake-os` as the login banner
(`/etc/issue`) instead of Buildroot's default welcome string.

#### Scenario: Login issue text

- **WHEN** an image is built
- **THEN** `/etc/issue` contains `Welcome to obake-os` and does not contain
  `Welcome to Buildroot`

### Requirement: Kernel release board and revision suffixes

Each image's kernel release string (`uname -r`) SHALL end with
`-g<shortsha>` or `-g<shortsha>-dirty`, where `<shortsha>` is the short git
commit of the image tree used for the build and `-dirty` is present when that
tree has uncommitted changes. Boards that define a LOCALVERSION suffix SHALL
include that suffix before the `-g…` segment: `-qdl` for `qdl`, `-generic`
for `generic-x86_64` and `generic-aarch64`. The `rk35xx` board SHALL omit a
board LOCALVERSION suffix for now.

#### Scenario: qdl kernel release shape

- **WHEN** a `qdl` image is built from a clean image git tree at commit
  `<shortsha>`
- **THEN** `uname -r` matches
  `<pinned-kernel-version>-qdl-g<shortsha>`

#### Scenario: generic board kernel release shape

- **WHEN** a `generic-x86_64` or `generic-aarch64` image is built from a clean
  image git tree at commit `<shortsha>`
- **THEN** `uname -r` matches
  `<pinned-kernel-version>-generic-g<shortsha>`

#### Scenario: rk35xx has no board suffix yet

- **WHEN** an `rk35xx` image is built from a clean image git tree at commit
  `<shortsha>`
- **THEN** `uname -r` matches
  `<pinned-kernel-version>-g<shortsha>` and does not contain `-rk35xx`

#### Scenario: Dirty tree marks the kernel

- **WHEN** an image is built from an image git tree with uncommitted changes
- **THEN** `uname -r` ends with `-dirty`
