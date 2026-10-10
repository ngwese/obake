# Proposal

## Why

Operators cannot apply signed updates over SSH as `obake`: the account lacks
limited sudo, while root is unlocked with an empty password so `su -` grants
full root. Removable update media also fails in practice — sticks are not
auto-mounted, and exFAT volumes error with `unknown filesystem type 'exfat'`.

## What Changes

- Grant the `obake` user passwordless sudo for a **minimal** command set needed
  for updates (at least `/usr/bin/obake-update`), not a general root shell.
- Lock the root account so empty-password `su -` and console root login fail.
- Auto-mount removable partitions for `obake` under `/media/<label>` (label from
  the filesystem; stable fallback when unlabeled).
- Enable in-kernel MS-DOS / FAT / FAT32 (VFAT) and exFAT so common USB sticks
  mount without FUSE hacks when the kernel drivers are available.
- Teach `obake-update` to prefer bundles already mounted under `/media`, so the
  update path does not need general `mount`/`umount` in sudoers.
- Cover the new behavior in headless guest checks where practical.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `host-config`: extend host-user privileged access for limited sudo and
  locked root; add removable-media auto-mount under `/media/<label>`.
- `os-update`: USB delivery uses auto-mounted media under `/media`; apply as
  `obake` via limited sudo without unlocking root.
- `os-image-build`: ship MS-DOS/FAT/FAT32 (VFAT) and exFAT in the shared base
  so every board can mount common removable filesystems.
- `image-verification`: assert root is locked and removable-media / sudo
  update access behave as specified in guest checks where feasible.

## Impact

- `image/os/buildroot/board/obake/rootfs-overlay/`: sudoers drop-in, root lock
  in post-fakeroot, udev/systemd (or equivalent) automount, `/media` layout,
  `obake-update` USB discovery against `/media`.
- `image/os/buildroot/configs/obake_base.config` and
  `board/obake/linux-obake.fragment` (plus board `linux.config` files): keep
  `CONFIG_FAT_FS` / `CONFIG_MSDOS_FS` / `CONFIG_VFAT_FS` and enable
  `CONFIG_EXFAT_FS`.
- `image/verification/guest/`: checks for locked root and `/media` automount
  or sudo update path.
- Docs (`image/docs/user-guide.md`, `os-update.md`, `host-config.md`).
- Partial WIP already in the tree (sudoers for `obake-update`, tmpfiles,
  path fixes for signing) is absorbed and completed by this change — including
  dropping any need for broad `mount`/`umount` sudo once automount lands.
- OTA: hosts pick up hardening and automount with the next slot image; no
  persist migration. Existing hosts that already have empty-password root stay
  that way until updated.
