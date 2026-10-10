# Tasks

## 1. Sudoers grant

- [x] 1.1 Update
      `image/os/buildroot/board/obake/rootfs-overlay/etc/sudoers.d/obake-update`
      to `obake ALL=(root) NOPASSWD: ALL` with comments that full sudo is for
      debugging and root stays locked; verify the file contains `NOPASSWD: ALL`
      and no longer lists only `/usr/bin/obake-update`
- [x] 1.2 Confirm `image/os/buildroot/board/obake/post-fakeroot.sh` still chmods
      `etc/sudoers.d` drop-ins to `0440` (no code change expected); verify by
      reading the existing `find ... chmod 0440` block

## 2. Guest verification and docs

- [x] 2.1 Change `check_update_sudo` in
      `image/verification/guest/obake-verify.sh` so
      `sudo -n /bin/sh -c true` must succeed (keep
      `sudo -n /usr/bin/obake-update status`); verify by reading the check that
      denial of shell sudo is no longer treated as success
- [x] 2.2 Update `image/docs/host-config.md`, `image/docs/os-update.md`, and
      `image/docs/user-guide.md` to describe passwordless sudo for all commands
      (including `sudo -s`) while root remains locked; verify no remaining
      "obake-update only" / "shell denied" wording for the sudo grant

## 3. Headless verification

- [x] 3.1 Rebuild and run the image verification path that boots a `VERIFY=1`
      guest (from `image/`) and confirm the guest `update-sudo` (or renamed)
      check passes with both `obake-update` and shell sudo succeeding, and that
      empty-password `su` to root still fails
