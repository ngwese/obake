# Tasks

## 1. Build entry point product flags

- [x] 1.1 Extend `image/build/build.sh` to require exactly one of
      `--installer` or `--update` on build invocations, reject missing/both
      with usage text, and leave `--print-dir` flag-free. Verify:
      `image/build/build.sh generic-x86_64` exits non-zero mentioning the
      flags; `image/build/build.sh --print-dir generic-x86_64` still prints
      a path.
- [x] 1.2 Implement product semantics and default development signing under
      `image/_build/signing` (create via `make-signing-material.sh` when
      absent): `--installer` forces `MAKE_INSTALLER=1` and default keyring /
      signing when unset; `--update` sets `MAKE_INSTALLER=0` and
      `MAKE_DISK=0` when unset and default signing when unset. Align
      `make-signing-material.sh`'s default outdir with `_build/signing` if
      needed. Verify: usage/`-h` or comments document the flags, and a
      dry inspection shows the exported defaults before the container runs
      (or a unit-style probe of the parsing block).

## 2. In-tree callers

- [x] 2.1 Update `image/verification/qemu/lib.sh` `obka_build` and every
      direct `bash "$buildsh"` call in
      `image/verification/qemu/{ssh,password,verify,install,install-interactive,reinstall,dev-vm,integration,health,update}-test.sh`
      to pass `--installer` or `--update` (disk-only stages that need
      `disk-*.img` without installer media use `--update` with
      `MAKE_DISK=1` when required). Verify: `rg 'bash "\$buildsh"' image/verification`
      shows a product flag on every non-`--print-dir` invocation.
- [x] 2.2 Update docs that invoke `image/build/build.sh` without a product
      flag: `image/README.md`, `image/verification/README.md`,
      `image/installer/README.md`, `image/docs/{user-guide,os-update,dev-qemu}.md`
      (and any other hits under `image/`). Prefer `--installer` for flash /
      installer examples and `--update` for the RAUC iteration examples;
      document default `_build/signing`. Verify:
      `rg 'image/build/build\.sh' image -g '*.md'` has no build example
      missing `--installer` or `--update` except `--print-dir` lines.

## 3. Headless verification

- [x] 3.1 Headlessly confirm product selection: missing flag fails; both
      flags fail; `--print-dir` succeeds. Verify: three commands as above
      with expected exit codes.
- [x] 3.2 Headlessly confirm an `--update` build for a board (reuse cache /
      existing work tree when possible) emits `obake-*-*.raucb` under the
      artifact dir, creates or reuses `image/_build/signing`, and does not
      require manually exported `RAUC_*`. Verify: artifact listing includes
      `.raucb` and signing files exist under `_build/signing`.
