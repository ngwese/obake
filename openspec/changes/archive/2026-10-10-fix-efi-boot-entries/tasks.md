# Tasks

## 1. Shared EFI slot-entry reconcile

- [x] 1.1 Add `image/os/scripts/efi-boot-entries.sh` that, given the target disk,
      ESP partition, and preferred slot, deletes every EFI entry labeled `A` or
      `B` except one active file-path entry per slot on that ESP
      (`\EFI\Linux\obake-a.efi` / `obake-b.efi`), creates either entry when it
      is missing, and prints or applies the resulting `BootOrder` (preferred
      slot, then the other slot, then non-slot entries in their previous
      order). When `BootNext` is set, leave order unchanged. Verify: the script
      exists and its header states the keep/delete rules above.
- [x] 1.2 Add a headless test under `image/verification/` that runs the helper
      against a stub `efibootmgr` whose listing has a stale `A` on another
      partition GUID, a `VenHw` `B`, one good `A`/`B` pair, and an unrelated
      entry. Verify: the test fails if the stale or `VenHw` entry remains, if
      the unrelated entry is deleted, or if the preferred slot is not first
      when `BootNext` is unset; the test passes with the stub.

## 2. Installer and boot helper

- [x] 2.1 Change `image/installer/install.sh` `install_boot` to call the helper
      for the ESP it just wrote, with the install's active slot preferred,
      instead of unconditional `efibootmgr -c` and `head -1`. Verify: `install.sh`
      no longer selects slot boot numbers with `head -1`, and a dry read shows
      it invokes the helper.
- [x] 2.2 Change
      `image/os/buildroot/board/obake/rootfs-overlay/usr/libexec/obake/efi-entries.sh`
      to call the same helper for the mounted `/boot/efi`, with the booted slot
      taken from `root=PARTLABEL=` on `/proc/cmdline`, and to skip order changes
      when `BootNext` is set. Verify: the boot helper no longer returns early
      just because some entry labeled `A` or `B` already exists.
- [x] 2.3 Note the single-entry rule in `image/docs/os-update.md` (and the
      installer README if it describes EFI entries). Verify: those docs no
      longer imply that each install appends another `A`/`B` pair.
