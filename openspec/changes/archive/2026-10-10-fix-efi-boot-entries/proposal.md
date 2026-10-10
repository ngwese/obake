# Proposal

## Why

RAUC selects the next slot by the first EFI boot entry whose label is `A` or
`B`. The installer creates a new `A` and `B` on every install and never removes
older ones, and the boot-time helper refuses to replace an existing label. After
a repartition or a repeated install, `BootNext` can point at an inactive entry
on a previous ESP, so an update that RAUC activated does not actually boot.

## What Changes

- On install, remove every existing EFI entry labeled `A` or `B`, create one
  active entry for each slot on the ESP just written, and set `BootOrder` to
  those new entries with the active slot first. Leave entries with any other
  label in place.
- On each boot, reconcile to the same pair for the mounted ESP: drop duplicate,
  inactive, `VenHw`, and wrong-disk `A`/`B` entries; keep or recreate one
  active file-path entry per slot. Do not clear `BootNext`. When `BootNext` is
  unset, order the kept slot entries with the booted slot first so the slot
  that just ran stays selected, then leave non-slot entries in their previous
  relative order.
- Cover the reconcile rules with a headless test that drives a stand-in
  `efibootmgr`, so a stale first `A` cannot be the entry an update activates.
- **BREAKING** for NVRAM on machines that already have duplicate `A`/`B`
  entries: the next install or the next boot of an image with this fix deletes
  those labels. Other firmware entries (for example an existing `UEFI OS` or
  another OS) are kept.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `system-install`: the installer leaves exactly one active EFI entry per slot
  on the target ESP and puts the active slot first, without accumulating
  duplicate `A`/`B` labels.
- `os-update`: slot activation uses the current ESP's single `A` and `B`
  entries, and a later boot keeps the booted slot selected when `BootNext` is
  unset.
- `image-verification`: a headless test fails if reconcile would keep a stale
  or duplicate slot entry ahead of the current ESP's entry.

## Impact

- `image/installer/install.sh` and
  `image/os/buildroot/board/obake/rootfs-overlay/usr/libexec/obake/efi-entries.sh`.
- A shared helper is acceptable if both call it; the observable result is one
  pair of entries and a stable boot order.
- OTA: hosts pick up the boot-time reconcile on the next booted slot. Persist,
  user data, and non-slot EFI entries are unchanged.
- Docs that describe EFI slot entries (`image/docs/os-update.md`, installer
  notes) should match the single-entry rule.
