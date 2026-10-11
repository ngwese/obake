# Design

## Context

See proposal.md for why the image git commit and the kernel release are
split. Today `image/os/buildroot/build.sh` turns
`image/os/scripts/image-git-rev.sh` into `OBAKE_KERNEL_LOCALVERSION`
(`-g<shortsha>[-dirty]`) and deletes linux package stamps when that string
changes. `image/os/buildroot/external.mk` passes it as the kernel
`LOCALVERSION` make variable. Board files
`image/os/buildroot/board/obake/generic-x86_64/linux.config` and
`image/os/buildroot/board/obake/generic-aarch64/linux.config` set
`CONFIG_LOCALVERSION="-generic"`. `rk35xx` has no board `linux.config`.
`image/os/buildroot/board/obake/post-build.sh` writes the same unsuffixed
release version into `VERSION`, `VERSION_ID`, and
`/etc/obake/image-version`.

The image git root is `image/`, which is what `image-git-rev.sh` already
uses. Build containers run as a different uid than the bind-mounted tree, so
that helper marks the image root as a safe directory.

## Goals / Non-Goals

**Goals:**

- Put image git provenance on `VERSION_ID` and `PRETTY_NAME`.
- Derive `-<board>-<rev>` from the first-seen history index of a per-board
  kernel-config hash, without writing the rev into the source tree.
- Rebuild the kernel when that local version changes, and leave it cached
  when only the image git commit changes.

**Non-Goals:**

- The kernel build timestamp (`uname -v`, `SOURCE_DATE_EPOCH`,
  `KBUILD_BUILD_TIMESTAMP`).
- Artifact directory names, RAUC bundle versions, `pinned-inputs.json`
  `image_version`, and `/etc/issue`.
- Adding the missing `qdl` board tree. The rev rule applies to `qdl` when
  that board is built.

## Decisions

### 1. Split os-release fields in post-build

`post-build.sh` keeps `VERSION` and `/etc/obake/image-version` as the release
version (`OBAKE_IMAGE_VERSION`, else `image/os/buildroot/VERSION`). It calls
`image-git-rev.sh` for `sha` and `dirty` and sets:

- `VERSION_ID=<version>-g<sha><dirty>` (`sha` is `nogit` when git cannot be
  read, which yields `-gnogit`)
- `PRETTY_NAME="obake-os <VERSION_ID>"`

`image-git-rev.sh` stays the source of the short hash, the dirty suffix, and
artifact-directory names.

### 2. History index in a new helper

Add `image/os/scripts/kernel-config-rev.sh`. It takes a board id and prints
`-<board>-<rev>`. It marks the image root as a safe directory, same as
`image-git-rev.sh`, then reads history. A shallow repository exits non-zero
and the build stops. When git metadata cannot be read at all, it prints
`-<board>-0` and the build continues; `VERSION_ID` still uses `-gnogit` from
`image-git-rev.sh`. `OBAKE_IMAGE_ROOT` overrides the image root so a
headless test can point at a temporary repository. The build does not set
that override.

For each board the hash inputs, in stable path order, are:

- `image/os/buildroot/board/obake/linux-realtime.fragment`
- `image/os/buildroot/board/obake/linux-obake.fragment`
- `image/os/buildroot/board/obake/<board>/linux.config` when that file exists
- The kernel-selection lines from `image/os/buildroot/configs/obake_base.config`
  and `image/os/buildroot/configs/obake-<board>.config`:
  `BR2_LINUX_KERNEL_USE_CUSTOM_CONFIG`,
  `BR2_LINUX_KERNEL_CUSTOM_CONFIG_FILE`,
  `BR2_LINUX_KERNEL_USE_ARCH_DEFAULT_CONFIG`,
  `BR2_LINUX_KERNEL_CONFIG_FRAGMENT_FILES`, and `BR2_LINUX_KERNEL_DTS*` /
  `BR2_LINUX_KERNEL_INTREE_DTS*`.

Excluded from the hash: blank lines, comment lines, every
`CONFIG_LOCALVERSION` line, and `BR2_LINUX_KERNEL_CUSTOM_VERSION` /
`BR2_LINUX_KERNEL_CUSTOM_VERSION_VALUE`. A kernel source-pin bump therefore
does not change `<rev>`.

The helper walks `git log --reverse` of those paths. Each commit's file
contents are normalized and hashed. Distinct hashes keep the order in which
they first appear. The working tree is hashed the same way. Its rev is the
1-based index of that hash, or one past the last index when the hash is new.
The helper never writes a rev file.

### 3. Inject the whole suffix as LOCALVERSION

`build.sh` exports `OBAKE_KERNEL_LOCALVERSION` from
`kernel-config-rev.sh` instead of `image-git-rev.sh`. `external.mk` still
appends it as the kernel `LOCALVERSION` make variable and still refuses to
build when it is unset. The existing output-directory stamp
(`obake-kernel-localversion`) still drops linux package stamps when the
string changes, so an image-only commit no longer forces a kernel rebuild.

`CONFIG_LOCALVERSION` in both generic `linux.config` files becomes empty
(`CONFIG_LOCALVERSION=""`). Those lines are excluded from the hash, so
clearing them does not itself advance `<rev>`. `rk35xx` continues to use the
architecture default config; its suffix comes only from `LOCALVERSION`.
`CONFIG_LOCALVERSION_AUTO` stays off.

### 4. Guest checks look at shape, not history

`image/verification/guest/obake-verify.sh` cannot see the build host's git
history. It checks field shape: `VERSION` equals `/etc/obake/image-version`,
`VERSION_ID` is that version plus the `-g` suffix, and `uname -r` ends with
the board id from `/etc/obake/board` and a positive integer. The history
index itself is checked by the headless helper test.

## Risks / Trade-offs

- [Rewriting image git history renumbers first-seen revs] → Accept it.
  The rev names the configuration's place in this history, not a content
  hash. A published image's `uname -r` stays stable as long as that history
  is not rewritten.
- [The first build of this scheme may not be rev 1] → Existing commits that
  touched the hashed inputs already occupy earlier indexes. No epoch
  constant is introduced.
- [A shared fragment change advances every board's rev] → Each board has its
  own sequence. Boards that are not rebuilt keep their previous modules
  directory until the next build.
- [A container uid cannot read the image git dir] → The helper adds
  `safe.directory` before it reads history, matching `image-git-rev.sh`.
  Without that, git looks missing and every container build would stamp
  `-<board>-0` and `-gnogit`.
- [Clearing `CONFIG_LOCALVERSION` while still passing the old git
  `LOCALVERSION` would double the suffix during a partial edit] → Both
  changes land together: empty `CONFIG_LOCALVERSION`, and `LOCALVERSION`
  set to the full `-<board>-<rev>`.

## Migration Plan

The next image build reports a new `uname -r` and a suffixed `VERSION_ID`.
`/lib/modules/<release>` follows `uname -r` once. `/etc/obake/image-version`
and RAUC bundle versions stay on the release version, so installed hosts
take the new identity only by installing a newly built image. No persistent
configuration migration.

Rollback is reverting the image tree and rebuilding. Hosts already installed
from an image with the new identity keep that identity until a different
image is installed.
