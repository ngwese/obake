# Design

## Context

See proposal.md. `image/installer/install.sh` `install_boot` always runs
`efibootmgr -c -L A` and `-L B`, then selects boot numbers with `head -1`,
which is the oldest label match. `efi-entries.sh` creates a label only when
that label is absent, and only rewrites `BootOrder` when it created an entry.
RAUC's EFI backend activates a slot by the first entry whose description equals
`bootname` (`A` / `B` in `image/os/rauc/system.conf`).

## Goals / Non-Goals

**Goals:**
- One active file-path entry per slot, on the ESP just installed or currently
  mounted
- Installer and boot helper share one reconcile implementation
- Headless test with a fake `efibootmgr`

**Non-Goals:**
- Deleting firmware entries that are not labeled `A` or `B`
- Changing RAUC's bootname matching or the slot-specific UKI command lines
- Rewriting `BOOTX64.EFI` / `BOOTAA64.EFI` (the firmware fallback copy stays as
  it is)

## Decisions

### 1. One helper, two callers

Add a small script, for example
`image/os/scripts/efi-boot-entries.sh`, that reads the current `efibootmgr -v`
listing and prints the `efibootmgr` operations to apply (delete these numbers,
create `A`/`B` if missing, set order). `install.sh` and `efi-entries.sh` call
it. The installer passes the target disk, partition, and active slot. The boot
helper passes the mounted ESP and the booted slot from `root=PARTLABEL=` on
`/proc/cmdline`.

**Alternative considered:** copy the same shell into both files. Rejected:
the `head -1` bug is easy to reintroduce in one of the copies.

### 2. What counts as the entry to keep

Keep an entry only when all of these hold:

- the label is exactly `A` or `B`
- `efibootmgr -v` shows it active
- the path is `\EFI\Linux\obake-a.efi` or `obake-b.efi` for that label
- the partition GUID in the entry is the ESP being installed or mounted

Delete every other `A` or `B`, including `VenHw(...)` entries and entries with
no active flag. If several entries match, keep the one that appears first in
`BootOrder`. If none match, create one with `efibootmgr -c` on that disk and
partition.

### 3. Boot order

Installer: after reconcile, `BootOrder` is the new active slot, then the other
slot, then every remaining non-slot entry in its previous order.

Boot helper: do nothing to order when `BootNext` is set. Otherwise put the
booted slot's kept entry first, then the other slot, then the remaining
non-slot entries in their previous order. Do not clear `BootNext`.

**Alternative considered:** never touch `BootOrder` from the boot helper.
Rejected: a stale entry at the front of `BootOrder` is what left
`Activated` on the previous slot after a one-shot `BootNext`.

### 4. Test with a fake efibootmgr

A host-side test invokes the helper with `PATH` pointed at a stub that replays
a fixture listing (duplicate `A` on an old GUID, a `VenHw` `B`, one good pair,
and an unrelated `UEFI OS` entry) and records `-B`, `-c`, and `-o`. Assert the
deletes, the surviving pair, the preserved unrelated entry, and that the booted
slot is first when `BootNext` is absent.

## Risks / Trade-offs

- **[Risk] Deleting `A`/`B` removes a hand-made entry with those labels** →
  Mitigation: those labels are reserved for the slot UKIs; other names are
  kept.
- **[Risk] Boot helper reorders before RAUC mark-good** → Mitigation: it runs
  once at boot, before `rauc.service`, and orders the slot that actually
  booted. RAUC's `BootNext` for a later switch is set at apply time, after
  this boot's helper has finished.
- **[Trade-off] Existing machines lose duplicate NVRAM entries on the first
  boot of the new image** → Intended.

## Migration Plan

- New installs get a clean pair immediately.
- An already-installed host gets the reconcile on the first boot of a slot
  that contains the new `efi-entries.sh`. Until that boot, duplicate entries
  remain.
- Rollback to a previous slot that still has the old helper stops the
  reconcile; it does not restore deleted NVRAM entries.

## Open Questions

None.
