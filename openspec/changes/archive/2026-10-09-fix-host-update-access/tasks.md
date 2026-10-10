# Tasks

## 1. Privileged access hardening

- [x] 1.1 Confirm `etc/sudoers.d/obake-update` grants only
      `/usr/bin/obake-update` (no `mount`/`umount`/shell) and remains mode
      `0440` via post-fakeroot. Verify: file contents and fakeroot chmod path.
- [x] 1.2 Fix `obake-update` non-root re-exec so it does not probe
      `sudo -n true`; re-exec with `sudo -n -- "$0" "$@"`. Verify: script no
      longer contains the `true` probe; dry logic review for apply/rollback.
- [x] 1.3 Lock the root account in `post-fakeroot.sh` by setting the root
      shadow password field to a locked marker. Verify: built or staged
      `etc/shadow` shows locked root; `obake` password path unchanged.

## 2. Removable media automount

- [x] 2.1 Add udev rule + `/usr/libexec/obake/media-automount` (or equivalent)
      that mounts removable/USB filesystems at `/media/<label>` with sanitized
      label and UUID/PARTUUID/kernel-name fallback. Verify: overlay files exist;
      rule matches add/remove for USB/removable FS partitions only.
- [x] 2.2 Ensure `/media` exists (tmpfiles or overlay) and mounts are usable by
      `obake` for reading update bundles. Verify: directory present in image;
      mount options or ownership documented in the helper.
- [x] 2.3 Teach `obake-update` to prefer `*.raucb` under mounted `/media` before
      any temporary mount probe; do not umount `/media` after install. Verify:
      discovery order in script; no umount of `/media/*` paths.

## 3. Removable filesystem support

- [x] 3.1 In `linux-obake.fragment` and board `linux.config` files that set
      FAT options, ensure `CONFIG_FAT_FS`, `CONFIG_MSDOS_FS`, and
      `CONFIG_VFAT_FS` remain `y`, and add `CONFIG_EXFAT_FS=y`. Verify: all
      four options are present in each of those configs.

## 4. Verification and docs

- [x] 4.1 Add guest checks that root is locked and that `obake` may
      `sudo -n /usr/bin/obake-update status` while shell sudo is denied.
      Verify: checks run in `obake-verify.sh` (or sibling) and fail on wrong
      shadow/sudoers.
- [x] 4.2 Update `image/docs/user-guide.md`, `os-update.md`, and host-config
      docs for limited sudo, locked root, and `/media/<label>` automount.
      Verify: docs mention the three behaviors and drop any implication that
      operators use empty-password root or manual `mount` for updates.
