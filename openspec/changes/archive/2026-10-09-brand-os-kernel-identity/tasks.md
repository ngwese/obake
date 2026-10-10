# Tasks

## 1. Kernel LOCALVERSION

- [x] 1.1 Set `CONFIG_LOCALVERSION="-qdl"` in
      `image/os/buildroot/board/obake/qdl/linux.config` and
      `CONFIG_LOCALVERSION="-generic"` in
      `generic-x86_64/linux.config` and `generic-aarch64/linux.config`.
      Leave `rk35xx` without a board LOCALVERSION. Verify: each of those
      three configs contains the expected `CONFIG_LOCALVERSION` line; no
      `rk35xx` board LOCALVERSION is introduced.
- [x] 1.2 Add a shared helper (used by `image/os/buildroot/build.sh`) that
      computes `-g<shortsha>[-dirty]` with the same rules as
      `image/build/build.sh` artifact directories (`nogit` when git is
      unavailable), exports it as `OBAKE_KERNEL_LOCALVERSION` /
      kernel `LOCALVERSION` (after board `CONFIG_LOCALVERSION`), and
      refreshes it on every build invocation. Ensure
      `CONFIG_LOCALVERSION_AUTO` is not enabled. Verify: build log shows
      the localversion; helper output matches artifact-dir shortsha logic.
- [x] 1.3 Rebuild (or partial linux rebuild) for `qdl` and confirm
      `/etc/obake/kernel` / modules directory name matches
      `<pinned>-qdl-g<shortsha>[-dirty]`. Verify: recorded kernel release
      string matches the required shape.

## 2. OS branding

- [x] 2.1 Set `BR2_TARGET_GENERIC_ISSUE="Welcome to obake-os"` in
      `image/os/buildroot/configs/obake_base.config`. Verify: merged
      defconfig / base config contains that string; `check-boards.sh`
      still passes.
- [x] 2.2 In `image/os/buildroot/board/obake/post-build.sh`, write
      `/usr/lib/os-release` (and keep `/etc/os-release` correct) with
      `NAME`/`ID=obake-os`, `VERSION`/`VERSION_ID` from the same source as
      `/etc/obake/image-version`, and `PRETTY_NAME="obake-os <version>"`.
      Fail the build if finalize left Buildroot identity. Verify: staged
      rootfs `os-release` and `/etc/issue` match the specs; version equals
      `image-version`.

## 3. Verification and docs

- [x] 3.1 Extend guest functional checks under `image/verification/guest/`
      to assert os-release fields, `/etc/issue`, and `uname -r` board +
      `-g` provenance shape for the board under test. Verify: checks fail
      if Buildroot identity or a missing `-g` suffix is present.
- [x] 3.2 Update `image/os/README.md` and `image/docs/user-guide.md` (and
      any other docs that claim a Buildroot login/os-release identity) for
      obake-os branding and the kernel release suffix rules. Verify: docs
      describe the new identity and do not instruct operators to expect
      "Welcome to Buildroot".
- [x] 3.3 Run a headless board build + QEMU guest path that exercises the
      new checks (at least one generic or `qdl` profile used in CI).
      Verify: harness reports pass for identity assertions.
