# Design

## Context

See proposal.md for motivation. Shared host options live in
`image/os/buildroot/board/obake/linux-obake.fragment`, applied to every board
via `BR2_LINUX_KERNEL_CONFIG_FRAGMENT_FILES` in `obake_base.config`. Today the
fragment enables SquashFS, ext4, tmpfs, overlay, FUSE, and FAT/exFAT; of those,
only `CONFIG_TMPFS_XATTR` (and `CONFIG_TMPFS_POSIX_ACL`) are set. Linux 7.2’s
Kconfig for this tree exposes:

- `CONFIG_SQUASHFS_XATTR` (optional; missing → the observed dmesg errors)
- `CONFIG_EXT4_FS_POSIX_ACL` and `CONFIG_EXT4_FS_SECURITY` (ext4 xattrs for
  ACLs / security labels; base xattr plumbing is otherwise built into ext4)
- `CONFIG_TMPFS_XATTR` / `CONFIG_TMPFS_POSIX_ACL` (already present)
- No separate xattr toggles for overlay, FUSE, FAT, or exFAT

A working-tree edit may already set `CONFIG_SQUASHFS_XATTR` in the fragment;
this change owns the complete set and verification.

## Goals / Non-Goals

**Goals:**
- One fragment change that turns on every available xattr-related option for
  onboard filesystems
- Clear comment in the fragment documenting what cannot be toggled
- Headless verification that the built kernel has the options (and preferably
  that a SquashFS SIF with xattrs no longer triggers the ignore/index errors)

**Non-Goals:**
- Installing userspace `attr` / `getfattr` packages unless needed for a check
- SELinux/AppArmor policy or enabling a MAC LSM
- Changing FAT/exFAT behavior or adding new filesystems
- Per-board special cases unless a base `linux.config` fights the fragment

## Decisions

### 1. Enable options only in `linux-obake.fragment`

Set in the shared fragment:

```
CONFIG_SQUASHFS_XATTR=y
CONFIG_EXT4_FS_POSIX_ACL=y
CONFIG_EXT4_FS_SECURITY=y
CONFIG_TMPFS_XATTR=y
CONFIG_TMPFS_POSIX_ACL=y
```

(`TMPFS_*` already present; keep them next to the new block for readability.)

**Alternative considered:** edit every board `linux.config`. Rejected: the
fragment is the cross-board contract and is already on every board’s fragment
list.

### 2. Treat overlay / FUSE / FAT as “no knob”

Overlay forwards xattrs from upper/lower; FUSE exposes xattrs via the FUSE
protocol; FAT/exFAT have no `*_XATTR` Kconfig in this kernel. Document that in
the fragment comment and the spec rather than inventing fake options.

### 3. Verify via built `.config` plus optional guest dmesg

Primary verify: after `linux-reconfigure` (or equivalent) for at least one
board, the assembled kernel `.config` contains the new symbols. Optional
stronger check: boot VERIFY and run a SIF known to carry xattrs (or the
embedded verify SIF if it has them) and assert dmesg does not contain the
SquashFS xattr ignore / id-index errors. Prefer a check that does not require
pulling `library://lolcow` on the build host if network is unavailable—config
presence is the must-have bar; guest dmesg is nice-to-have if a local SIF
suffices.

**Alternative considered:** only document and rely on operator retest with
lolcow. Rejected for the config assertion; guest dmesg remains best-effort.

## Risks / Trade-offs

- **[Risk] Slight kernel size / surface increase** → Mitigation: small;
  options are standard on general-purpose kernels; needed for real SIFs.
- **[Risk] Board base config overrides fragment** → Mitigation: confirm
  `.config` after merge; if a board clears an option, add an explicit line in
  that board’s `linux.config`.
- **[Trade-off] EXT4_FS_SECURITY without SELinux** → Acceptable: still needed
  for `security.*` xattrs (e.g. file capabilities) used by containers.

## Migration Plan

- Rebuild board images; OTA delivers the new kernel on the next signed slot.
- No persist or shape changes; no rollback beyond the previous slot’s kernel.

## Open Questions

None that block specs or tasks. Whether the VERIFY fixture SIF contains xattrs
can be settled during implementation when choosing the guest assertion.
