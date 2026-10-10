# Tasks

## 1. Bundle gate

- [x] 1.1 In `image/build/build.sh`, default `MAKE_BUNDLE=0` for `--installer`
      and `MAKE_BUNDLE=1` for `--update` when unset, leaving an explicit
      `MAKE_BUNDLE` in place; verify `grep MAKE_BUNDLE image/build/build.sh`
      shows both defaults
- [x] 1.2 In `image/os/buildroot/board/obake/post-image.sh`, build `.raucb`
      files only when `MAKE_BUNDLE=1` and signing material is set; verify
      `grep MAKE_BUNDLE image/os/buildroot/board/obake/post-image.sh` gates
      the bundle loop
- [x] 1.3 After staging in `image/build/build.sh`, remove `*.raucb` from the
      output directory when `MAKE_BUNDLE` is not 1, and remove installer
      images other than the one this build wrote (`installer-<console>.img`);
      verify the script deletes those paths before `print_artifacts`

## 2. Staging progress

- [x] 2.1 Print a progress line in `image/build/build.sh` before the
      cache-to-host copy and before the derived-image delete on the cache
      volume; verify both lines are present and are not redirected to
      `/dev/null`
- [x] 2.2 In `image/docs/user-guide.md`, describe `*.raucb` as an `--update`
      artifact; verify the artifact table no longer says bundles appear
      whenever signing is set

## 3. Headless check

- [x] 3.1 Run `image/build/build.sh --print-dir qdl` and a dry read of the
      product branches (or `bash -n image/build/build.sh` and
      `bash -n image/os/buildroot/board/obake/post-image.sh`); verify both
      scripts parse and `--print-dir` still prints a path without building
