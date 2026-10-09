# Tasks

## 1. Build tree and entry point

- [x] 1.1 `git mv` `image/verification/build.sh`,
      `image/verification/build-env/`, and
      `image/verification/build-singularity-stage.sh` into `image/build/`;
      update their internal path arithmetic (`here`, `project_root`) and every
      reference to `verification/build-env`; verify `bash -n` on the moved
      scripts and that `image/build/build-env/run.sh bash -c true` runs
- [x] 1.2 Change `image/build/build.sh` to `build.sh <board> [profile]` (no
      arch argument) and add a `--print-dir` mode that prints the artifact
      directory without building; verify `--print-dir generic-x86_64` prints
      the derived path without building and an unknown board fails with a
      usage error
- [x] 1.3 Update the verification callers to the moved entry point and new CLI:
      `image/verification/qemu/lib.sh` (`obka_arch_dir`, `obka_build`) and
      `image/verification/reproducibility.sh`; verify `bash -n` on each and
      that `lib.sh` still resolves an artifact directory via `--print-dir`
- [x] 1.4 Update `image/README.md` and `image/verification/README.md` to the
      new `image/build/` location and board/profile CLI; verify every build
      command quoted in those files runs as written (headless, `--print-dir`
      where a build is not desired)

## 2. Board profiles and implied architecture

- [x] 2.1 Add `image/os/buildroot/profiles/<board>/{default,virtual}.conf` for
      `generic-aarch64`, `generic-x86_64`, `qdl`, and `rk35xx` with the
      installer defaults from `design.md`; verify each file is syntactically
      sourceable and the generic boards' two profiles are identical
- [x] 2.2 Source the selected profile in `image/os/buildroot/build.sh`,
      defaulting to `default`, letting an explicit environment setting win, and
      failing on an unknown profile; verify a build reports the profile values
      and `INSTALLER_CONSOLE=serial build.sh qdl` overrides the profile
- [x] 2.3 Make the target architecture follow the board (no arch argument);
      verify each board builds for its own architecture and no caller passes an
      arch
- [x] 2.4 Document the profiles and the captured bare-metal flow in
      `image/installer/README.md` and `image/docs/user-guide.md`; verify the
      documented `qdl` `default` installer command runs as written

## 3. Repository-local, user-owned outputs

- [x] 3.1 Stage artifacts under `image/_out/` and the Buildroot work under
      `image/_build/`, move harness outputs to `image/_verify/` and dev-vm state
      to `image/_dev/`, and update `image/.gitignore` to ignore `_*`; verify a
      build creates directories only inside `image/` and `git status` stays
      clean
- [x] 3.2 Name the artifact directory
      `_out/os/<board>/<profile>/<version>-<shortsha>[-dirty]/` using
      `image/os/buildroot/VERSION` and the image git revision, with a non-git
      fallback; verify two boards of one architecture and the `virtual`/`default`
      profiles of one board produce distinct paths, and a dirty tree adds
      `-dirty`
- [x] 3.3 Run the container as the invoking user (`--user "$(id -u):$(id -g)"`)
      and bootstrap the named cache volume's ownership once; verify a
      containerized build stages files owned by the invoking user and that the
      output directory can be removed without elevated privileges, with a
      scoped `chown` fallback if the platform does not preserve ownership
- [x] 3.4 Stop staging Buildroot intermediates (`.installer`, `.disk-work`,
      `.initramfs`) and print `artifact: <path>` for each staged file plus the
      directory on completion; verify the staged directory contains only
      produced artifacts and the printed paths exist
- [x] 3.5 Update the output-path references in `image/docs/dev-qemu.md`,
      `image/docs/os-update.md`, and `image/installer/README.md` to the new
      `_out`/`_verify`/`_dev` locations; verify each quoted path matches a file
      a build actually produces

## 4. Integration verification

- [x] 4.1 Clean-build `generic-x86_64` and `qdl` for both profiles headlessly;
      assert the artifact paths, user ownership, profile-selected defaults
      (`MAKE_INSTALLER`, `INSTALLER_CONSOLE`, `INSTALLER_TARGET_DISK`), and
      printed artifact locations
- [x] 4.2 Run the QEMU installer and boot harnesses on `generic-x86_64` against
      the new `_out` artifact directory headlessly and confirm they pass
      unchanged, and confirm `image/verification/reproducibility.sh` still
      prints `OBKA_REPRODUCIBLE_PASS`
