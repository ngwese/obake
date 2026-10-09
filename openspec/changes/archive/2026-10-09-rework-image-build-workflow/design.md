# Design

## Context

See `proposal.md`. Today the build wrapper is `image/verification/build.sh`,
with the container at `image/verification/build-env/`. It takes `[arch] [outdir]`
and reads the board from `OBAKE_BOARD`; the arch only names the output directory
(`$project_root/dist/os/<arch>` at the top-level repo, outside `image/`). The
container runs as root, so `docker cp` stages root-owned files, and it hardcodes
`LAYOUT=partition-layout.verify.json` even when building an installer for real
hardware. The low-level `image/os/buildroot/build.sh <board>` already knows the
board and derives arch in `post-image.sh`, but has no profile concept. Boards
are `generic-aarch64`, `generic-x86_64`, `qdl`, and `rk35xx`, each a board
fragment under `image/os/buildroot/configs/`.

## Goals / Non-Goals

**Goals:**

- Put build tooling in its own tree, decoupled from verification tooling.
- Capture per-board installer defaults in named profiles.
- Deterministic, repository-local, user-owned, non-colliding output paths.
- A CLI that takes only a board (and optional profile).

**Non-Goals:**

- Changing image content, kernels, package set, or installer logic.
- Changing `partition-layout.json` / `partition-layout.verify.json` contents.
- Changing which partition layout a build targets; layout selection is
  unchanged and out of scope.
- Adding or removing boards, or changing shape/OTA/persistent-config behavior.
- Changing what verification proves; only where its I/O lives.

## Decisions

1. **Build tree at `image/build/`.** Move `build.sh`, `build-env/`, and
   `build-singularity-stage.sh` there. Verification keeps `qemu/`, `guest/`,
   `verify-env/`, `reproducibility.sh`, `reproducibility/`, and
   `update-server.sh`. Rationale: the harnesses already consume an artifact
   directory, so the move is structural and reinforces the existing
   "decoupled build and verification" contract. Alternative: leave the wrapper
   in place and only document decoupling; rejected because the placement is the
   confusion the change exists to remove.

2. **Profiles as shell fragments under `image/os/buildroot/profiles/<board>/`.**
   `default.conf` and `virtual.conf` hold `KEY=value` assignments for
   `MAKE_INSTALLER`, `INSTALLER_CONSOLE`, and `INSTALLER_TARGET_DISK`. The
   low-level `image/os/buildroot/build.sh` sources the selected profile
   (defaulting to `default`) and exports the values, so both the containerized
   wrapper and a direct host build get the same defaults. An explicit
   environment setting wins over the profile; an unknown profile fails.
   Rationale: the low-level builder is the one place that already knows the
   board, and it is the common path for both wrappers. Alternative: store
   profiles as JSON and parse with `python3`; shell sourcing avoids a parser and
   matches the existing knob environment style.

3. **Profile contents (settles the open calls).**
   - `generic-aarch64`, `generic-x86_64`: identical `default`/`virtual` —
     `MAKE_INSTALLER=0`, `INSTALLER_CONSOLE=serial`,
     `INSTALLER_TARGET_DISK=/dev/vdb`. Generic boards are the
     emulation/verification target, so both profiles are the same, and a plain
     build should not assemble an installer. Installer harnesses keep opting in
     with `MAKE_INSTALLER=1`.
   - `qdl` `virtual`: `MAKE_INSTALLER=1`, `INSTALLER_CONSOLE=serial`,
     `INSTALLER_TARGET_DISK=/dev/vdb`. `qdl` `default`: `MAKE_INSTALLER=1`,
     `INSTALLER_CONSOLE=local`, `INSTALLER_TARGET_DISK=interactive`. This is
     exactly the bare-metal flow documented in `user-guide.md` chapter 5, now
     captured instead of retyped.
   - `rk35xx` both profiles: `MAKE_INSTALLER=0`. Rationale: the hardware has not
     been acquired, and the installer medium does not carry the Rockchip raw
     boot region, so an installer is neither verified nor installable on this
     board today; the disk image is still produced. If that changes, `default`
     can set `MAKE_INSTALLER=1`.

4. **CLI `build.sh <board> [profile] [--] [make-args]`.** The arch is no longer
   an argument: the board fragment's `BR2_aarch64`/`BR2_x86_64` already selects
   it, and the output path is keyed by board. A `--print-dir` mode prints the
   artifact directory for the board/profile without building, which the
   verification harnesses use to locate (and decide whether to rebuild)
   artifacts. Alternative: keep an optional arch argument for compatibility;
   rejected because it recreates the redundancy the change removes. This is a
   breaking CLI change, so every in-tree caller is updated in the same change.

5. **Output tree and path.**
   ```
   image/_build/                                       Buildroot work tree + dl cache
   image/_out/os/<board>/<profile>/<version>-<sha>[-dirty]/   staged artifacts
   image/_verify/...                                   harness logs and work
   image/_dev/...                                      dev-vm state
   ```
   `<version>` is `image/os/buildroot/VERSION`; `<sha>` is
   `git -C image rev-parse --short HEAD`, suffixed `-dirty` when the image tree
   has uncommitted changes (with a non-git fallback). Only the staged
   top-level artifacts are kept; Buildroot's internal `output/` stays in
   `_build`. Rationale: board and profile in the path remove the same-arch
   collision; per-revision subdirectories keep successive builds. Alternative:
   a flat `_out/<board>-<profile>/`; rejected because it either overwrites or
   needs manual cleanup.

6. **User-owned outputs.** Run the build container as the invoking user
   (`--user "$(id -u):$(id -g)"`), and keep the named cache volume but bootstrap
   its ownership once (a root helper that `chown`s it when the user cannot write
   it). `docker cp` then stages files with the cache files' uid/gid, which are
   the invoking user's. The build's root hack `FORCE_UNSAFE_CONFIGURE=1` is kept
   (Buildroot tolerates it and the bootstrap still runs as root). Rationale:
   Buildroot prefers a non-root build, and this makes the container path match
   the direct host path. Alternatives: `chown -R` after `docker cp` (works but
   does not "map" ownership, and is O(files) on every build); a bind-mounted
   cache (slower on macOS Docker, where a named volume is deliberate).

7. **Stop staging intermediates and print artifacts.** Staging copies only the
   produced files (or omits `.installer`, `.disk-work`, `.initramfs`), not the
   Buildroot scratch tree, and the wrapper prints `artifact: <path>` for each
   staged file plus the directory. Rationale: the scratch directories were the
   bulk of the root-owned clutter; printing satisfies discoverability.

8. **Migrate callers and docs in the same change.** `qemu/lib.sh` maps
   `OBKA_ARCH` to a board and asks `build.sh --print-dir`; the QEMU harnesses
   build the `virtual` profile and read from the returned directory; harness
   outputs move under `_verify/`; `reproducibility.sh` calls the moved wrapper.
   Docs (`image/README.md`, `image/verification/README.md`,
   `image/installer/README.md`, `image/docs/{user-guide,dev-qemu,os-update}.md`)
   are updated to the new structure, CLI, and output paths.

## Risks / Trade-offs

- **[Risk] A package step fails under a non-root container user.** → Buildroot
  is designed to build as a non-root user; keep `FORCE_UNSAFE_CONFIGURE=1`, and
  verify with a full clean build of at least `generic-x86_64` and `qdl`.
- **[Risk] The existing `obake-buildroot-cache` volume is root-owned.** →
  Bootstrap its ownership once at startup (`test -w` then a root `chown`); note
  that migrating the first time costs one `chown`.
- **[Risk] `docker cp` ownership differs on rootless or desktop Docker.** →
  Keep a final `chown -R` fallback scoped to the artifact directory when the
  staged files are not owned by the user.
- **[Risk] The CLI change breaks existing scripts and muscle memory.** →
  Update every in-tree caller and the docs in the same change; `--print-dir`
  gives scripts a stable way to discover paths.
- **[Risk] Existing root-owned `dist/` at the top-level repo cannot be removed
  by the user.** → The change stops creating it; document a one-time manual
  removal (it is outside the image repo and cannot be cleaned automatically).
- **[Trade-off] A profile-per-path adds a directory level.** → Accepted; it is
  what makes `virtual` and `default` coexist for one board.
- **[Limitation] Layout selection is unchanged.** → The wrapper keeps its
  current layout choice, so `qdl` `default` does not yet build with the release
  layout. Selecting the release layout is deferred to a follow-up.

## Migration Plan

1. `git mv` the build tools into `image/build/` and update their internal paths.
2. Add the profile files and the resolution/precedence logic to the low-level
   builder; switch the wrapper to the board/profile CLI and `--print-dir`.
3. Point the shell and script references at `_build`/`_out` and update
   `.gitignore` to ignore `_*`.
4. Run the container build as the user with the cache-ownership bootstrap.
5. Update `qemu/lib.sh`, the harnesses, `reproducibility.sh`, and the docs.
6. Verify: build `generic-x86_64` and `qdl` (both profiles) and confirm the
   paths, ownership, printed locations, profile defaults, and that the QEMU
   installer/boot harnesses pass against the new artifact directory.

Rollback is a revert: the moved tools return to `verification/`, the profiles
are removed, and the `_*` directories can be deleted by the invoking user.

## Open Questions

- Whether the one-time `dist/` cleanup should be a documented manual step or a
  best-effort helper invoked with `sudo` (the change assumes a documented manual
  step). The singularity stage builder is kept with the build tools, since it
  shares their build environment, and can be retired separately.
