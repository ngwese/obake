# Spec Delta

## MODIFIED Requirements

### Requirement: Custom realtime kernel

Each image SHALL include a realtime kernel built from a pinned kernel source
with a project realtime configuration fragment, and SHALL record that kernel's
release in the image. The recorded release identifies the kernel configuration
revision and SHALL NOT encode the image git revision.

#### Scenario: Realtime tuning present in image

- **WHEN** an image is built
- **THEN** the included kernel is built from the pinned source with the
  project's realtime configuration, and the image's recorded kernel release
  matches `uname -r` and does not encode the image git revision

### Requirement: obake-os release identity

The built image SHALL present as `obake-os` in `/etc/os-release`: `NAME` and
`ID` SHALL be `obake-os`; `VERSION` SHALL equal the contents of
`image/os/buildroot/VERSION` (or `OBAKE_IMAGE_VERSION` when set); `VERSION_ID`
SHALL be that version followed by `-g<shortsha>`, `-g<shortsha>-dirty`, or
`-gnogit`; and `PRETTY_NAME` SHALL be `obake-os` followed by `VERSION_ID`.
`<shortsha>` is the short git commit of the image tree. `-dirty` SHALL be
present when that tree has uncommitted changes. `-gnogit` SHALL be used when
the image git metadata cannot be read.

#### Scenario: os-release fields

- **WHEN** an image is built from a clean image git tree at commit `<shortsha>`
- **THEN** `/etc/os-release` has `NAME=obake-os`, `ID=obake-os`, `VERSION`
  matching the image version, `VERSION_ID` equal to `<version>-g<shortsha>`,
  and `PRETTY_NAME="obake-os <version>-g<shortsha>"`

#### Scenario: Aligns with image-version

- **WHEN** an image is built
- **THEN** os-release `VERSION` equals the contents of
  `/etc/obake/image-version`, and `VERSION_ID` begins with that same version
  followed by `-g`

#### Scenario: Dirty tree marks VERSION_ID

- **WHEN** an image is built from an image git tree with uncommitted changes
- **THEN** `VERSION_ID` ends with `-dirty` and `VERSION` equals
  `/etc/obake/image-version` with no dirty suffix

#### Scenario: Missing git metadata

- **WHEN** an image is built and the image git metadata cannot be read
- **THEN** `VERSION_ID` ends with `-gnogit` and `VERSION` equals
  `/etc/obake/image-version`

## REMOVED Requirements

### Requirement: Kernel release board and revision suffixes

**Reason**: The kernel release no longer carries the image git commit. Board
id and a config-history rev replace the `-g<shortsha>[-dirty]` suffix, and
`rk35xx` gains a board suffix.

**Migration**: Use the kernel release board and config revision requirement.
Image git provenance moves to os-release `VERSION_ID`.

## ADDED Requirements

### Requirement: Kernel release board and config revision

Each image's kernel release string (`uname -r`) SHALL end with
`-<board>-<rev>`, where `<board>` is the board id and `<rev>` is a positive
integer when image git history can be read, and `0` when it cannot. The
board id SHALL be `generic-x86_64`, `generic-aarch64`, `rk35xx`, or `qdl`
for those boards. The release string SHALL NOT contain the image git commit
and SHALL NOT end with `-dirty`.

#### Scenario: generic-x86_64 kernel release shape

- **WHEN** a `generic-x86_64` image is built from a clean image git tree
- **THEN** `uname -r` matches
  `<pinned-kernel-version>-generic-x86_64-<rev>` where `<rev>` is a positive
  integer

#### Scenario: generic-aarch64 kernel release shape

- **WHEN** a `generic-aarch64` image is built from a clean image git tree
- **THEN** `uname -r` matches
  `<pinned-kernel-version>-generic-aarch64-<rev>` where `<rev>` is a positive
  integer

#### Scenario: rk35xx kernel release shape

- **WHEN** an `rk35xx` image is built from a clean image git tree
- **THEN** `uname -r` matches `<pinned-kernel-version>-rk35xx-<rev>` where
  `<rev>` is a positive integer

#### Scenario: qdl kernel release shape

- **WHEN** a `qdl` image is built from a clean image git tree
- **THEN** `uname -r` matches `<pinned-kernel-version>-qdl-<rev>` where
  `<rev>` is a positive integer

#### Scenario: Release omits image git

- **WHEN** an image is built
- **THEN** `uname -r` does not contain an image git short commit and does not
  end with `-dirty`

#### Scenario: Missing git metadata uses rev 0

- **WHEN** an image is built and the image git metadata cannot be read
- **THEN** `uname -r` ends with `-<board>-0`

### Requirement: Kernel config revision from history

`<rev>` SHALL be the 1-based index of the board's current kernel-configuration
hash in the ordered set of distinct hashes of that board's kernel
configuration inputs across the image git history, oldest first. Inputs are
the board kernel configuration, the shared kernel fragments, and the board's
kernel config selection. The local-version suffix and the pinned kernel source
version are not inputs. The same hash SHALL select the same rev, including
when the configuration returns to an earlier hash. A commit that does not
change the hash SHALL NOT change `<rev>`.

#### Scenario: Unchanged config keeps the rev

- **WHEN** an image is rebuilt after a commit that does not change that
  board's kernel configuration inputs
- **THEN** `uname -r` is unchanged

#### Scenario: New config hash advances the rev

- **WHEN** a board's kernel configuration inputs change to a hash not yet
  present in the image git history and that change is committed
- **THEN** `<rev>` is one greater than the previous maximum rev for that board

#### Scenario: Reverted config reuses its rev

- **WHEN** a board's kernel configuration inputs return to a hash already
  present in the image git history
- **THEN** `<rev>` equals the rev previously assigned to that hash

#### Scenario: Kernel source pin does not bump the rev

- **WHEN** only the pinned kernel source version changes
- **THEN** `<rev>` is unchanged and the leading version in `uname -r` is the
  new pinned version

### Requirement: Dirty kernel config uses the next rev

When the image tree has uncommitted changes and the working-tree kernel
configuration hash has never been committed, the build SHALL use the next rev
for that build and SHALL NOT write that rev into the source tree. A dirty
tree whose hash is already in history SHALL use that hash's rev.

#### Scenario: Unseen dirty hash

- **WHEN** an image is built from a dirty image tree whose kernel
  configuration hash is not in git history
- **THEN** `<rev>` is one greater than the maximum committed rev for that
  board, and the build does not modify the source tree

#### Scenario: Dirty tree with a known hash

- **WHEN** an image is built from a dirty image tree whose kernel
  configuration hash matches one already in history
- **THEN** `<rev>` equals the rev of that hash

### Requirement: Shallow image git fails the build

The build SHALL fail when the image git checkout is shallow, so a truncated
history cannot assign a kernel rev.

#### Scenario: Shallow checkout

- **WHEN** an image build is started from a shallow image git checkout
- **THEN** the build fails before producing an image
