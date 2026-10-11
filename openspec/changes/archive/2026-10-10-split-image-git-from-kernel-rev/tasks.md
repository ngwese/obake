# Tasks

## 1. Kernel config revision helper

- [x] 1.1 Add `image/os/scripts/kernel-config-rev.sh` that prints
  `-<board>-<rev>` for a board id. Hash the inputs and exclusions in
  design.md (shared fragments, optional board `linux.config`, kernel
  selection lines; drop comments, blanks, `CONFIG_LOCALVERSION`, and the
  kernel source-pin lines). Assign `<rev>` as the 1-based first-seen index
  of that hash in `git log --reverse` of the image repo (`image/`), mark
  that repo `safe.directory`, honor `OBAKE_IMAGE_ROOT`, exit non-zero on a
  shallow repo, print `-<board>-0` when git metadata cannot be read, and
  do not write the source tree. Verify: `bash -n image/os/scripts/kernel-config-rev.sh`
  succeeds.

- [x] 1.2 Add a headless test `image/os/scripts/kernel-config-rev-test.sh`
  that builds a temporary git repo via `OBAKE_IMAGE_ROOT` and checks: the
  first config hash is rev 1; a new hash is rev 2; restoring an earlier
  hash reuses its rev; a dirty unseen hash uses the next rev and leaves
  the repo unmodified; a dirty known hash reuses that rev; a comment-only
  edit and a `CONFIG_LOCALVERSION` or kernel source-pin edit do not bump
  the rev; a shallow clone exits non-zero; a directory that is not a git
  repo prints `-<board>-0`. Verify: `image/os/scripts/kernel-config-rev-test.sh`
  exits 0.

## 2. Kernel local version

- [x] 2.1 In `image/os/buildroot/build.sh`, set `OBAKE_KERNEL_LOCALVERSION`
  from `kernel-config-rev.sh` for the board being built, before `make`, and
  keep the output-directory stamp that drops linux package stamps only when
  that string changes. Do not ignore a non-zero exit from the helper.
  Verify: `bash -n image/os/buildroot/build.sh` succeeds, the script calls
  `kernel-config-rev.sh`, and it no longer takes the kernel local version
  from `image-git-rev.sh`.

- [x] 2.2 Set `CONFIG_LOCALVERSION=""` in
  `image/os/buildroot/board/obake/generic-x86_64/linux.config` and
  `image/os/buildroot/board/obake/generic-aarch64/linux.config`. Leave
  `CONFIG_LOCALVERSION_AUTO` unset. Update the local-version comment in
  `image/os/buildroot/board/obake/linux-obake.fragment` and the
  `LOCALVERSION` comment in `image/os/buildroot/external.mk` so they
  describe `-<board>-<rev>` instead of the image git suffix. Verify: both
  `linux.config` files contain `CONFIG_LOCALVERSION=""`, neither contains
  `CONFIG_LOCALVERSION="-generic"`, and `external.mk` still appends
  `LOCALVERSION=$(OBAKE_KERNEL_LOCALVERSION)`.

- [x] 2.3 Update the kernel release description in `image/os/README.md` and
  `image/docs/user-guide.md` so `uname -r` is documented as
  `<pinned-kernel-version>-<board>-<rev>`, including `-rk35xx-<rev>`, with
  `<rev>` defined as the history index of the kernel configuration. Verify:
  both docs describe that form and neither still says the kernel release
  ends in `-g<shortsha>`.

## 3. os-release git suffix

- [x] 3.1 In `image/os/buildroot/board/obake/post-build.sh`, keep `VERSION`
  and `/etc/obake/image-version` as the release version, and set
  `VERSION_ID` and `PRETTY_NAME` from `image/os/scripts/image-git-rev.sh`
  (`<version>-g<sha><dirty>`, or `-gnogit` when sha is `nogit`). Verify:
  `bash -n image/os/buildroot/board/obake/post-build.sh` succeeds, and the
  script writes `VERSION` from the release version and `VERSION_ID` with
  the `-g` suffix.

- [x] 3.2 Update `check_os_identity` and `check_kernel_identity` in
  `image/verification/guest/obake-verify.sh` so `VERSION` must equal
  `/etc/obake/image-version`, `VERSION_ID` must be that version plus
  `-g<shortsha>`, `-g<shortsha>-dirty`, or `-gnogit`, `PRETTY_NAME` must be
  `obake-os` plus `VERSION_ID`, and `uname -r` must end with the board id
  from `/etc/obake/board` and a positive integer, with no image git commit
  in the release string. Verify: `bash -n image/verification/guest/obake-verify.sh`
  succeeds, and the kernel check no longer requires a `-g` segment in
  `uname -r`.

- [x] 3.3 Update the os-release description in `image/os/README.md` and
  `image/docs/user-guide.md` so `VERSION` matches `/etc/obake/image-version`
  and `VERSION_ID` / `PRETTY_NAME` carry the `-g<shortsha>[-dirty]` suffix.
  Verify: both docs describe that split and neither still says `VERSION_ID`
  is only the unsuffixed release version.
