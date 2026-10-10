# Design

## Context

See proposal.md for motivation. The RO root already mounts writable `/var`
(tmpfs) and `/home` (partition). `/media` was only ensured as an empty
directory via tmpfiles on the RO root. Automount uses
`/usr/libexec/obake/media-automount` from udev `RUN+` under systemd 258
(`PrivateMounts=yes` on `systemd-udevd.service`). On hardware, insert fails at
`mkdir /media/<label>` with EROFS before any mount attempt.

## Goals / Non-Goals

**Goals:**
- Writable `/media` mount root early in boot
- Volume mounts visible in the host mount namespace for `obake` and Singularity
  binds of `/media`
- Keep udevd `PrivateMounts` unless a host-NS handoff is impossible
- Stronger guest check that would have caught the RO `/media` bug

**Non-Goals:**
- Moving the public path away from `/media/<label>`
- Full USB-insert automount coverage in default QEMU (hardware-dependent)
- Dropping udevd isolation as the primary fix
- udisks2 / desktop storage stack

## Decisions

### 1. tmpfs on `/media` (same pattern as `/var`)

Add to `etc/fstab` (or an equivalent `.mount` unit):

```
tmpfs  /media  tmpfs  mode=0755,nosuid,nodev,noexec  0  0
```

Rationale: mountpoint directories must be creatable on an RO root; tmpfs is
already used for `/var`; content under `/media` is ephemeral by nature.
Alternatives: bind a persist subdir (wrong lifetime), use only `/run/media`
(breaks published `/media` path for docs and Singularity binds).

Update `etc/obake/directory-contract` and docs to name `/media` as volatile.

Keep `tmpfiles.d/obake-media.conf` for `/run/obake-media` state; the `d /media`
line remains harmless once tmpfs is mounted (ensures mode).

### 2. Host-NS mount via systemd, not `mount` in udev `RUN+`

Keep udev matching USB/removable filesystem partitions, but hand off add/remove
to a oneshot template (e.g. `obake-media-automount@.service`) via
`ENV{SYSTEMD_WANTS}` / `TAG+="systemd"`, or call `systemd-mount` so PID 1
performs the mount in the host namespace.

Prefer reusing the existing helper as `ExecStart=` of the oneshot (add/remove
args) so label sanitization and FAT uid/gid logic stay in one place.

Rationale: satisfies host visibility without setting `PrivateMounts=no` on
udevd. Alternatives considered: disable PrivateMounts (weaker udevd isolation);
keep `RUN+`+`mount` (fails host visibility).

### 3. Udev match: `add|change` with already-mounted guard

Match `ACTION=="add|change"` for filesystem partitions so late `ID_FS_*`
population is not missed. The helper (or unit) MUST no-op when the device is
already mounted at the chosen `/media/<name>` to avoid remount loops.

### 4. Failure visibility

Stop discarding all mount/mkdir stderr in the helper when invoked from
systemd (journal captures unit stdout/stderr). Keep early exits for
non-filesystem devices.

### 5. Verification

Extend `check_media_automount` in `obake-verify.sh` to require `/media` is a
mountpoint and that `sudo` can create/remove a probe directory under it.
Keep presence checks for the helper and udev rule. Full USB insert remains
hardware-dependent; document that gap.

## Risks / Trade-offs

- **[tmpfs hides baked-in /media content]** → Image must not ship needed files
  under `/media`; only mountpoints. Current overlay has none.
- **[change events remount loop]** → Guard on existing mount / state file
  before mount.
- **[SYSTEMD_WANTS vs RUN+ races]** → Unit must tolerate device still settling;
  helper already re-probes with blkid.
- **[EFI + data partitions both mount]** → Expected; both appear under
  `/media/<label>`. No special skip of ESP-labeled volumes unless a later
  change needs it.

## Migration Plan

- Next OTA/slot image includes fstab (or mount unit), udev/systemd handoff,
  helper tweaks, contract/docs, and guest check.
- No persist migration. Existing hosts keep broken automount until updated.
- Rollback: previous slot image without tmpfs `/media`.
