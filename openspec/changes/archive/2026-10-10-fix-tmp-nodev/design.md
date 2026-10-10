# Design

## Context

See proposal.md for motivation. systemd 258 (pinned via Buildroot
`2026.02.3` / `efi_stub.systemd_version`) installs `tmp.mount` with a static
`local-fs.target.wants/` symlink. The unit's `Options=` are
`mode=1777,strictatime,nosuid,nodev,size=50%,nr_inodes=1m` plus a graceful
`usrquota` option. The obake root is read-only, so this tmpfs is what makes
`/tmp` writable. `fstab` mounts `/var` and `/media` only; it does not mention
`/tmp`.

SingularityCE 4.5.1 (`internal/pkg/build/build.go`) checks the mount that
holds the build temporary directory, which defaults to `/tmp`. `nodev` there
is a warning; `noexec` is a hard failure. The sandbox under that directory
needs device nodes (bootstrap and fakeroot image conversion). `obake-verify.sh`
already runs a local SIF and `exec --fakeroot`, which do not take the build
path.

## Goals / Non-Goals

**Goals:**
- `/tmp` stays a separate volatile tmpfs, writable, `nosuid`, without `nodev`
  or `noexec`.
- Default Singularity builds (no `TMPDIR` / `SINGULARITY_TMPDIR`) stop warning
  and can create device nodes in the sandbox.
- Guest verify proves the mount flags and that a local fakeroot build does
  not emit the warning.

**Non-Goals:**
- Relocating Singularity's temporary directory to `/var`, `/home`, or a
  dedicated path.
- Changing `/var` or `/media` mount flags (`/media` stays `nodev,noexec`).
- Masking `tmp.mount` or replacing `/tmp` with a symlink.
- Shape definition changes, network `Bootstrap: docker` builds in CI, or
  installer-only behavior.

## Decisions

### 1. Drop-in on the vendor `tmp.mount`, not an fstab line

Add a rootfs-overlay drop-in at
`usr/lib/systemd/system/tmp.mount.d/obake.conf` that replaces `Options=`
with the vendor list minus `nodev`:

```
mode=1777,strictatime,nosuid,size=50%%,nr_inodes=1m,x-systemd.graceful-option=usrquota
```

Write `50%%` in the unit file so systemd's specifier rules yield a literal
`size=50%`. Keep `nosuid`, the size and inode caps, `strictatime`, and the
graceful `usrquota` option. Do not add `noexec`.

The overlay is applied after the systemd package, so the drop-in sits beside
the vendor unit. `/usr` is on the read-only root, outside the `/etc` overlay,
so a persist reset cannot put `nodev` back.

**Alternative considered:** an `fstab` line, matching `/var` and `/media`.
Rejected: systemd already pulls in `tmp.mount` via a static wants symlink.
A generated unit would be a second definition of the same mount and can lose
to the vendor unit. A drop-in edits the unit that actually starts.

**Alternative considered:** `SINGULARITY_TMPDIR=/var/tmp` (that tmpfs has no
`nodev`). Rejected: the default path would still warn, `sudo` and explicit
`/tmp` would miss the variable, and large builds would share the `/var`
tmpfs with persisted state.

**Alternative considered:** symlink `/tmp` → `/var/tmp` so `tmp.mount`'s
`ConditionPathIsSymbolicLink=!/tmp` skips the mount. Rejected: `/tmp` would
inherit `/var`'s flags (no `nosuid`) and would compete with `/var` for space.

### 2. Prove it with mount flags plus one offline fakeroot build

Extend `image/verification/guest/obake-verify.sh` (run as `obake`):

- `findmnt` shows `/tmp` is `tmpfs`, options include `nosuid`, and do not
  include `nodev` or `noexec`.
- `singularity build --fakeroot` of a definition whose `Bootstrap` is
  `localimage` and `From` is `/opt/obake/verify.sif`, with no temporary
  directory override, completes, and its combined output does not contain
  the `nodev` warning. Write the definition and the output SIF under `/tmp`.

Do not assert the exact size or inode options, so a later systemd default
tweak does not fail the harness if `nosuid` is still set and `nodev` /
`noexec` are still absent.

**Alternative considered:** only `findmnt`. Rejected: the warning is what
operators see, and a flag check can pass while Singularity still resolves
a different temporary directory.

**Alternative considered:** `mknod` on `/tmp` as root. Weaker than the
warning check and does not exercise fakeroot. The build covers both.

### 3. Directory contract names `/tmp` as volatile

Add a line to `etc/obake/directory-contract` and a row in
`image/docs/host-config.md`: `/tmp` is a volatile tmpfs, discarded on boot,
`nosuid`, device nodes allowed. Do not add it to `/etc/obake/persisted-var`.

## Risks / Trade-offs

- **[Risk] Without `nodev`, a process that holds `CAP_MKNOD` can create
  device nodes on `/tmp`** → `obake` does not hold that capability. `nosuid`
  stays, so a setuid binary planted on `/tmp` is not honored. Passwordless
  sudo already exists; `nodev` was not the privilege boundary.
- **[Risk] The drop-in replaces the whole `Options=` string and can drift
  from a future systemd unit** → Guest check locks the flags that matter
  (`tmpfs`, `nosuid`, no `nodev`, no `noexec`). Revisit the drop-in when
  the systemd pin moves.
- **[Trade-off] Builds still use a 50% RAM tmpfs** → Same cap as today's
  `tmp.mount`. A build that needs more space still fails for lack of space;
  this change does not move it to disk.
- **[Risk] A full SIF assembly in QEMU adds verify time** → One localimage
  build from the existing tiny verify SIF, no registry access.

## Migration Plan

The new drop-in ships in the OS image. `/tmp` is recreated at boot, so an
OTA needs no persist migration and leaves `/home` alone. Rollback is the
previous slot, which mounts `/tmp` with `nodev` again.

## Open Questions

None.
