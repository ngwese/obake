# Tasks

## 1. Apply password and SSH password auth at rootfs assembly

- [x] 1.1 In `image/os/buildroot/board/obake/post-fakeroot.sh`, when
      `OBKA_PASSWORD` is non-empty, set a hashed password for `obake` on the
      target rootfs without printing the cleartext; when unset or empty, leave
      the users-table `*` behavior unchanged — verify by inspecting
      `TARGET_DIR/etc/shadow` after a fakeroot/post-build pass or a full image
      build (hash present vs `*`/`!`)
- [x] 1.2 When `OBKA_PASSWORD` is non-empty, ensure sshd allows password
      authentication (e.g. write `PasswordAuthentication yes` under
      `image/os/buildroot/board/obake/` rootfs `sshd_config.d` from
      post-build/post-fakeroot); when unset, leave sshd config unchanged —
      verify the drop-in (or equivalent) is present only on password builds
- [x] 1.3 Keep `image/os/buildroot/board/obake/users.txt` password field as
      `*` and update its comment to note the optional `OBKA_PASSWORD`
      override — verify the file still documents key-only default

## 2. Forward the build knob

- [x] 2.1 Forward `OBKA_PASSWORD` through `image/build/build.sh`,
      `image/build/build-env/run.sh`, and
      `image/verification/verify-env/run.sh` like other `OBKA_*` knobs —
      verify the variable name appears in each forward list and a containerized
      build sees the value when set on the host

## 3. Docs and headless verification

- [x] 3.1 Document `OBKA_PASSWORD` in `image/docs/user-guide.md` (SSH access
      section) and `image/os/README.md` (userland), including that a set
      password enables SSH password authentication as `obake`, default
      key-only behavior, image-only (not persist) scope, and that the hash
      is recoverable from the artifact — verify the docs name the env var
      and an example `OBKA_PASSWORD=... image/build/build.sh …` invocation
- [x] 3.2 Add a headless check under `image/verification/` that builds with a
      test `OBKA_PASSWORD`, boots under QEMU, and logs in as `obake` over SSH
      using that password (password auth, not the authorized key); also
      assert a default build keeps password login disabled — verify the
      script exits 0 headlessly
