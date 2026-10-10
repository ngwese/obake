# Design

## Context

See proposal.md for motivation. Today Buildroot hardcodes `/usr/lib/os-release`
at target finalize (`NAME=Buildroot`, …) and defaults
`BR2_TARGET_GENERIC_ISSUE` to `Welcome to Buildroot`. The realtime kernel is
built from a pinned tarball (`pins.json`); board configs set
`BR2_LINUX_KERNEL_CUSTOM_CONFIG_FILE` (or arch default for `rk35xx`) plus shared
fragments. No `CONFIG_LOCALVERSION` is set. Artifact staging already computes
`<version>-<shortsha>[-dirty]` from the image git tree in
`image/build/build.sh`. Post-build records `/etc/obake/image-version` from
`VERSION` / `OBAKE_IMAGE_VERSION`. UKIs already pass
`--os-release=@$TARGET_DIR/etc/os-release` into `ukify`.

## Goals / Non-Goals

**Goals:**

- Make user-visible OS identity `obake-os` (os-release + login banner).
- Make `uname -r` encode board LOCALVERSION (where defined) and image-tree
  git provenance, matching artifact naming semantics.
- Keep board suffixes in board-owned kernel config; inject git provenance at
  build time without forking the shared base.

**Non-Goals:**

- Changing `image/os/buildroot/VERSION` content or artifact directory layout.
- Adding an `rk35xx` board LOCALVERSION (deferred).
- Using the upstream kernel git tree for `CONFIG_LOCALVERSION_AUTO` (kernel
  input is a tarball).
- Renaming hostname, `/etc/obake/*` identity files, or RAUC compatible strings
  beyond what os-release/UKI embedding already implies.

## Decisions

### 1. Board suffix via `CONFIG_LOCALVERSION` in the board kernel config

- **Choice:** Set `CONFIG_LOCALVERSION` in each board's `linux.config`:
  `-qdl` for `qdl`, `-generic` for both generic boards. Leave `rk35xx` unset
  (arch-default config + fragments only).
- **Why:** Matches “suffix specified in the board file” and keeps check-boards
  base/board split clean (LOCALVERSION lives in board-owned kernel config, not
  `obake_base.config`).
- **Alternatives:** A side-car `localversion` text file per board (extra
  indirection); encoding the suffix only in Buildroot board fragments (would
  not apply cleanly to full custom `linux.config` boards).

### 2. Git provenance via the kernel `LOCALVERSION` make variable

- **Choice:** Export `OBAKE_KERNEL_LOCALVERSION=-g<shortsha>[-dirty]` from
  `build.sh` (shared helper `image/os/scripts/image-git-rev.sh`, same rules as
  artifact directories) and pass it into the kernel build as
  `LINUX_MAKE_ENV += LOCALVERSION=...` in `external.mk`. Disable
  `CONFIG_LOCALVERSION_AUTO`.
- **Why:** `scripts/setlocalversion` concatenates
  `KERNELVERSION` + `localversion*` files + `CONFIG_LOCALVERSION` +
  `LOCALVERSION` env. Putting git in a `localversion-*` file placed the board
  suffix last (`…-gSHA-qdl`); the make variable appends after
  `CONFIG_LOCALVERSION`, yielding `…-qdl-gSHA[-dirty]`.
- **Alternatives:** `localversion-*` file (wrong order); full
  `CONFIG_LOCALVERSION` generated at configure time (moves board suffix out of
  the board `linux.config`).

### 3. os-release overwrite after Buildroot finalize write

- **Choice:** In `post-build.sh` (after Buildroot writes os-release, or
  re-write both `/usr/lib/os-release` and the `/etc/os-release` link target),
  emit the obake-os fields from the same version source as
  `/etc/obake/image-version`.
- **Why:** Buildroot's Makefile hardcodes Buildroot identity; there is no
  useful base-config knob for NAME/ID on the pinned release. Post-build already
  owns image version recording.
- **Alternatives:** Rootfs overlay static os-release (loses dynamic VERSION);
  patching Buildroot (high maintenance).

### 4. Login banner via `BR2_TARGET_GENERIC_ISSUE`

- **Choice:** Set `BR2_TARGET_GENERIC_ISSUE="Welcome to obake-os"` in
  `obake_base.config` (shared by every board).
- **Why:** That is what drives `/etc/issue` today; os-release alone will not
  change the login line.
- **Alternatives:** Overlay `/etc/issue` only (works if ISSUE is emptied so
  Buildroot does not overwrite); post-build rewrite of `/etc/issue`.

### 5. Verification in existing guest functional checks

- **Choice:** Extend the guest integration (or equivalent) checks to assert
  os-release fields, `/etc/issue`, and `uname -r` shape for the board under
  test.
- **Why:** Matches the image-verification delta and existing headless pattern
  (`/etc/obake/image-version`, `uname -r` already sampled in update tests).

## Risks / Trade-offs

- **[Risk] Rootfs/UKI change every commit (and when dirty)** → Accept: identity
  clarity over cross-commit bit-identical rootfs. Pin selection remains
  deterministic; same commit + clean tree should still reproduce.
- **[Risk] post-build vs finalize ordering** → Confirm on pinned Buildroot that
  post-build runs after os-release generation (or write in post-fakeroot /
  re-touch after); add a post-build assertion that NAME is `obake-os`.
- **[Risk] Incremental kernel rebuild ignores a newly dirty tree** → Ensure the
  localversion file is refreshed on each build invocation, not only on
  defconfig.
- **[Risk] Modules path / DKMS assumptions on uname -r** → No out-of-tree
  modules in-tree today; `/etc/obake/kernel` already records the built release
  from `lib/modules`.
- **[Trade-off] `rk35xx` bare of board suffix** → Documented deferral; git
  suffix still applies so revision remains visible.

## Migration Plan

- New images only; no migration of running hosts beyond a normal OTA/install.
- After apply, rebuild a board image and confirm `cat /etc/os-release`,
  `cat /etc/issue`, and `uname -r` (or `/etc/obake/kernel`) match the specs.
- No RAUC key or slot-layout change; updates continue to replace the whole
  slot including the new identity.
