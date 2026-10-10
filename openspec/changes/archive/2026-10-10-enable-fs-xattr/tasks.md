# Tasks

## 1. Kernel fragment xattr options

- [x] 1.1 In `image/os/buildroot/board/obake/linux-obake.fragment`, enable
      `CONFIG_SQUASHFS_XATTR`, `CONFIG_EXT4_FS_POSIX_ACL`, and
      `CONFIG_EXT4_FS_SECURITY`, keep `CONFIG_TMPFS_XATTR` and
      `CONFIG_TMPFS_POSIX_ACL`, and add a short comment that overlay/FUSE/FAT
      have no separate host xattr toggles. Verify: `grep` on the fragment shows
      all five symbols set to `y` and the comment is present.
- [x] 1.2 If any board `linux.config` under
      `image/os/buildroot/board/obake/` explicitly disables one of those
      options, align it with the fragment (or remove the conflicting line).
      Verify: no board `linux.config` sets those symbols to `n` or leaves a
      `# CONFIG_* is not set` that would clear them after merge.

## 2. Headless rebuild check

- [x] 2.1 Run `linux-reconfigure` (or equivalent) for at least one board via
      `image/build/build.sh` so the fragment is applied, then confirm the
      assembled kernel `.config` in the Buildroot output contains
      `CONFIG_SQUASHFS_XATTR=y`, `CONFIG_EXT4_FS_POSIX_ACL=y`,
      `CONFIG_EXT4_FS_SECURITY=y`, and `CONFIG_TMPFS_XATTR=y`. Verify: those
      four lines appear in the board’s `build/linux-*/.config` (or equivalent
      path under the build cache/work tree).
- [x] 2.2 Optionally boot a VERIFY image and mount a SquashFS-backed SIF that
      contains xattrs; confirm dmesg has neither “Xattrs in filesystem, these
      will be ignored” nor “unable to read xattr id index table”. Skip if no
      suitable local SIF is available without network. Verify: when run, guest
      or host dmesg after the mount lacks those two strings.
