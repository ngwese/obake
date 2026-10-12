# Tasks

## 1. Build-time credential inputs

- [x] 1.1 Replace `realpath -m` in `image/build/build.sh` with a portable
  absolute-path helper (plain `realpath`, else `cd "$(dirname)" && pwd -P`),
  and verify `image/build/build.sh --print-dir qdl` succeeds on macOS and a
  container build reaches Buildroot on both macOS and Linux.
- [x] 1.2 Stage the file named by `OBKA_SSH_AUTHORIZED_KEY` into the gitignored
  `image/_build/seeds/` directory and rewrite the variable to its `/work` path
  before launching the container; fail with an error naming
  `OBKA_SSH_AUTHORIZED_KEY` when the path is not a readable file. Verify a
  build with a key outside the repository succeeds and a bogus path fails.
- [x] 1.3 Confirm `image/build/build-env/run.sh` forwards the staged key path
  into the container and that `image/os/buildroot/board/obake/make-installer.sh`
  embeds the seed at `/opt/obake/authorized_key.pub`. Verify by inspecting the
  staged installer rootfs for the key.
- [x] 1.4 Document the host-path and platform behavior of
  `OBKA_SSH_AUTHORIZED_KEY` in `image/docs/user-guide.md` and `build.sh --help`,
  and verify the documented invocation runs as written.

## 2. Install-time seeding

- [x] 2.1 Extend `seed_user` in `image/installer/install.sh` to write the
  embedded seed to `~/.ssh/authorized_keys` on the user partition (0700 `.ssh`,
  0600 key, `obake`-owned, uid/gid read from the passwd database) only when no
  key is present. Verify with a QEMU install that the user partition holds the
  key with the expected mode and owner, and that no key appears when the medium
  carries no seed.
- [x] 2.2 Ensure a `PRESERVE_USER=1` reinstall leaves an existing
  `~/.ssh/authorized_keys` unchanged. Verify by reinstalling over a disk whose
  user partition already carries a key.
- [x] 2.3 Stop seeding `/etc/ssh/authorized_keys.d/obake` in
  `image/installer/install.sh` and verify the persist partition no longer
  contains that file after install.
- [x] 2.4 Update `image/installer/README.md` and `image/docs/user-guide.md` for
  install-time key seeding and verify the documented flow installs a key.
- [x] 2.5 Seed the user partition in `image/os/scripts/make-disk.sh` from the
  build seed (`mke2fs -d` under `fakeroot`, `obake` ownership), passing the
  `obake` uid/gid from `post-image.sh`. Verify `disk-a.img`'s user partition
  contains the key with the expected owner and mode.

## 3. Runtime self-service

- [x] 3.1 Stop `image/os/buildroot/board/obake/post-build.sh` seeding the rootfs
  `/etc/ssh/authorized_keys.d/obake`, keeping the `AuthorizedKeysFile` path in
  `rootfs-overlay/etc/ssh/sshd_config.d/obake.conf` only as an empty fallback.
  Verify a built rootfs contains no seeded authorized key.
- [x] 3.2 Confirm the setuid busybox `passwd` lets `obake` change an existing
  password without sudo (Buildroot installs `/usr/bin/passwd` setuid and
  busybox marks it SUID_REQUIRE) and that a key-only account's first password
  needs elevation; verify both on a booted image.
- [x] 3.3 Confirm the home `.ssh` directory and key permissions satisfy sshd
  `StrictModes` for a user-created key. Verify `obake` writes
  `~/.ssh/authorized_keys` over SSH and a subsequent key login succeeds.
- [x] 3.4 Update `image/docs/host-config.md` and `image/docs/user-guide.md` for
  home-based authorized keys and self-service password change, and verify the
  documented commands run as written.

## 4. Verification harness

- [x] 4.1 Extend `image/verification/qemu/ssh-test.sh` to assert seeded-key
  login, in-place key replacement retiring the seed, and persistence of the
  replaced key across a slot switch; verify the script passes headlessly.
- [x] 4.2 Extend `image/verification/qemu/password-test.sh` to assert `obake`
  changes an existing password without sudo and that the changed password
  survives a slot switch, and verify it passes headlessly.
- [x] 4.3 Extend `image/verification/guest/obake-persist-test.sh` and the guest
  checks for credential persistence on the user partition, and verify the
  headless run reports pass.

## 5. Integration

- [x] 5.1 Run the end-to-end flow: build an installer with a key outside the
  build tree, install under QEMU, log in with the seed, replace the key, change
  the password, roll back to the other slot, and confirm the new credentials
  remain; verify via the QEMU harness output.
- [x] 5.2 Reconcile `image/docs/dev-qemu.md` and any spec cross-references with
  the new seeding and self-service behavior, and verify the documented dev flow
  works as written.
