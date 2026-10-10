# Design

## Context

See proposal.md for motivation. Today `obake-update apply` still mounts USB
itself under `/run/obake-usb.*`, so any non-root path would need `mount`/
`umount` in sudoers. WIP already ships `etc/sudoers.d/obake-update` for
`/usr/bin/obake-update` and a sudo re-exec in the client, but the re-exec
probes `sudo -n true`, which fails under a narrowly scoped grant. Root is
created by Buildroot with an empty password (not listed in `users.txt`), so
`su -` as `obake` succeeds. Removable media has no automount path; the kernel
fragment enables FAT/MSDOS/VFAT but not exFAT.

## Goals / Non-Goals

**Goals:**
- Passwordless, narrowly scoped sudo so `obake` can run `obake-update` only
- Locked root (no empty-password `su` / console login)
- Always auto-mount removable filesystems for `obake` at `/media/<label>`
- In-kernel MS-DOS/FAT/FAT32 (VFAT) and exFAT so common sticks mount
  without FUSE
- USB bundle discovery prefers already-mounted `/media` trees
- Guest checks for root lock and scoped sudo where practical

**Non-Goals:**
- Full desktop storage stack (udisks2, GVFS, desktop notifications)
- Encrypting or passphrase-unlocking removable volumes
- Changing SSH key/password policy for `obake` beyond root hardening
- Auto-applying updates when media is inserted (still user-initiated)
- Migrating already-deployed hosts that keep empty root until they OTA

## Decisions

### 1. Sudoers grant is only `obake-update`

Keep `etc/sudoers.d/obake-update` as:

```
obake ALL=(root) NOPASSWD: /usr/bin/obake-update
```

Do **not** add `mount`/`umount`/`su`/`bash`. After automount, apply still needs
root for RAUC and `/persist` health state; that stays inside the single
allowed binary via re-exec.

Fix the client probe: replace `sudo -n true` with a direct
`exec sudo -n -- "$0" "$@"` (or equivalent that does not require an unrelated
NOPASSWD command). Mode `0440` and root ownership stay enforced in
`post-fakeroot.sh`.

### 2. Lock root in post-fakeroot

In `post-fakeroot.sh`, set the root shadow password field to a locked marker
(`!` or `*`) after Buildroot generates `etc/shadow`. Do not invent a root
password. Leave `obake` password handling (`OBKA_PASSWORD`) unchanged.

### 3. Lightweight udev + helper automount (not udisks)

Ship a small `/usr/libexec/obake/media-automount` (or equivalent) driven by a
udev rule on block add/remove for removable / USB-attached filesystem
partitions. Rationale: Buildroot image stays small; no D-Bus storage daemon;
behavior is fully under our control for `/media/<label>`.

Mount path rules:
- Prefer `ID_FS_LABEL` (sanitized: reject `/`, `..`, empty after sanitize)
- Fallback: `ID_FS_UUID` or `PARTUUID`, else `disk-<kernel-name>`
- Create `/media/<name>`, mount read-write with ownership usable by `obake`
  (for VFAT/exFAT: `uid`/`gid` of the `obake` account; for POSIX FS: rely on
  mode/ACL or `chmod` so `obake` can read update bundles)
- On remove: `umount` and remove the mountpoint directory when empty
- Skip internal disks (root, persist, ESP, slot partitions) — only removable
  or USB-bus block devices with a filesystem

QEMU `usb-storage` often reports `RM=0`; match `ID_BUS=usb` as well as
removable so verification and lab USB work.

### 4. FAT family and exFAT via in-kernel drivers

Ensure `linux-obake.fragment` and the board `linux.config` files that
duplicate FAT settings enable the full removable FAT set plus exFAT:

- `CONFIG_FAT_FS=y`
- `CONFIG_MSDOS_FS=y`
- `CONFIG_VFAT_FS=y` (FAT32 with long names; also used for the ESP)
- existing NLS options already required for VFAT (`CONFIG_NLS_CODEPAGE_437`,
  `CONFIG_NLS_ISO8859_1`)
- `CONFIG_EXFAT_FS=y` (new where missing)

Userspace `mount` from util-linux is enough once the kernel knows the types;
avoid a FUSE exfat/fat stack unless a board kernel cannot enable these
options.

### 5. `obake-update` prefers `/media`

Change USB discovery to:
1. Scan mounted trees under `/media` for `*.raucb` (maxdepth as today)
2. Optionally fall back to the existing root-only `mount`/`umount` probe for
   partitions that automount missed (still fine: apply already runs as root)

Do not umount `/media` mounts after install — those are system-managed.

### 6. Verification

Extend guest functional checks to assert:
- root shadow field starts with `!` or `*`
- `sudo -n /usr/bin/obake-update status` succeeds as `obake`
- `sudo -n /bin/sh -c true` (or similar) is denied

Full USB insert automount may be awkward in the default QEMU disk layout;
prefer a unit/script presence check plus, if the harness already attaches
USB, a mountpoint assertion. Document any gap in tasks if hardware-only.

## Risks / Trade-offs

- **[Label collisions]** Two sticks with the same label → second mount fails or
  overwrites path. Mitigation: if target is busy, append a short unique suffix
  (UUID prefix) rather than mounting over an existing mount.
- **[Path injection]** Untrusted labels could contain `../`. Mitigation:
  sanitize to a safe directory name before mkdir/mount.
- **[Scoped sudo + broken probe]** Until the `sudo -n true` bug is fixed,
  apply still fails for `obake` even with the drop-in present — fix is
  mandatory in the same change.
- **[Locked root ops]** Recovery that relied on empty-password root must use
  SSH as `obake`, physical console with a set `OBKA_PASSWORD`, or reimage.
- **[exFAT kernel age]** Very old board kernels without `CONFIG_EXFAT_FS` would
  need a different approach; current obake boards use configs that can take
  the fragment option.
