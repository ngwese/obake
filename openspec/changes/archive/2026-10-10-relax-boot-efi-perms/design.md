# Design

## Context

See proposal.md for motivation. Today `etc/fstab` mounts `LABEL=OBKA_ESP` at
`/boot/efi` with `umask=0077`, so VFAT synthesizes owner-only modes. The
`/boot` and `/boot/efi` directories are created in `post-build.sh` via
`install -d` without an explicit mode, so a restrictive build umask can leave
them `0700`. Guest checks already run as `obake` and assert the ESP is
mounted for RAUC, but they do not assert unprivileged access.

## Goals / Non-Goals

**Goals:**
- Unprivileged read/list of `/boot/efi` for `obake`
- Traversable `/boot` prefix so `df` and path resolution succeed
- Keep ESP writes root-only (RAUC / `efi-entries` unchanged)
- Guest check that would have caught the current `df` failure

**Non-Goals:**
- Changing ESP ownership away from root, or granting write to `obake`
- Removable-media automount (`fix-media-automount` stays separate)
- Altering RAUC slot layout, UKI paths, or `obake-efi-entries`
- Making the ESP read-only (updates still need write as root)

## Decisions

### 1. ESP mount: `umask=0022`

Change fstab from:

```
LABEL=OBKA_ESP  /boot/efi  vfat  umask=0077  0  2
```

to:

```
LABEL=OBKA_ESP  /boot/efi  vfat  umask=0022  0  2
```

Rationale: default mount uid/gid remain 0, so files appear root-owned with
world-readable modes; others still lack write. Matches the intent used for
removable VFAT in `media-automount` (`umask=022`) while keeping system
ownership. Alternatives considered: `dmask=0022,fmask=0133` (more precise,
same outcome); leave `umask=0077` and only fix `/boot` (fixes some `stat`
failures but not `ls`/`cd` into the ESP).

### 2. Explicit `0755` on `/boot` and `/boot/efi` in post-fakeroot

After `post-build.sh` creates the mountpoints, set mode in
`post-fakeroot.sh` (where ownership/mode is authoritative in the image):

```
install -d -m 0755 "${target_dir}/boot" "${target_dir}/boot/efi"
# or chmod 0755 / chown 0:0 on those paths
```

Rationale: fakeroot is the existing place for root-owned mode enforcement
(sudoers, shadow). Relying on build umask is brittle. Keep `post-build.sh`
`install -d` for path creation; harden mode in fakeroot.

### 3. Guest check: list `/boot/efi` as `obake`

Add `check_esp_access` (name flexible) to `obake-verify.sh` that:
- requires `/boot/efi` mounted (reuse or call alongside `check_rauc_status`)
- succeeds only if `test -x /boot && test -r /boot/efi && ls /boot/efi`
  works without sudo
- optionally asserts a non-privileged write probe fails (documents the
  write-still-denied half of the requirement)

Wire into the existing `run` list. No new QEMU harness needed; ESP is
already present in the default disk layout.

## Risks / Trade-offs

- **[Unprivileged read of UKIs]** → Acceptable: loaders are not secrets;
  `obake` already has full sudo. Mitigation: keep root ownership so write
  still requires privilege.
- **[OTA hosts keep old umask until slot update]** → Expected; document in
  migration. Rollback: previous slot restores `umask=0077`.
- **[Over-broad world-read on ESP]** → Same as typical `umask=0022` VFAT;
  no network-exported ESP path on this product.

## Migration Plan

- Next OTA/slot image ships fstab + fakeroot mode + guest check.
- No persist migration. Live hosts keep the old ESP umask until updated.
- Rollback: boot previous slot.
