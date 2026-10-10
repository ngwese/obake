# Tasks

## 1. Image mount and directory modes

- [x] 1.1 In `image/os/buildroot/board/obake/rootfs-overlay/etc/fstab`,
      change the `OBKA_ESP` VFAT options from `umask=0077` to `umask=0022`.
      Verify: `grep` shows `umask=0022` and no `umask=0077` on that line.
- [x] 1.2 In `image/os/buildroot/board/obake/post-fakeroot.sh`, ensure
      `/boot` and `/boot/efi` are root-owned `0755` after mountpoint creation.
      Verify: the script contains an explicit `0755` (or equivalent) for both
      paths; `post-build.sh` still creates the directories.
- [x] 1.3 If `image/docs/host-config.md` (or related image docs) imply an
      owner-only ESP, update them to describe unprivileged read/list by
      `obake` with root-only write. Verify: docs match the new fstab policy.

## 2. Guest verification

- [x] 2.1 Add `check_esp_access` (or equivalent) to
      `image/verification/guest/obake-verify.sh` that fails unless `obake` can
      list `/boot/efi` without sudo, and wire it into the `run` list. Verify:
      the check function exists and is invoked from the main suite.
- [x] 2.2 Optionally assert in that check that a non-sudo write under
      `/boot/efi` fails. Verify: the write probe is present or intentionally
      omitted with a one-line comment if the suite cannot safely probe.

## 3. Headless confirmation

- [x] 3.1 Rebuild/reinstall (or OTA) an image that includes the change and run
      the guest functional suite (e.g. `image/verification` path that
      exercises `obake-verify.sh`). Verify: the new ESP-access check passes
      and `df` as `obake` does not report Permission denied for `/boot/efi`.
