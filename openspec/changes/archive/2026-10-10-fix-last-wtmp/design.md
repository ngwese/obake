# Design

## Context

See proposal.md for motivation. Observed on the current Buildroot image:

- `/var` is tmpfs (`fstab`); only paths listed in `/etc/obake/persisted-var`
  are bind-mounted from `/persist/var` by `obake-persist-var.service`.
- Today that list includes `log/journal` but not the rest of `/var/log`.
- systemd's packaged `tmpfiles.d/var.conf` creates `/var/log` only; unlike
  Debian, it does not create `wtmp`/`btmp`/`lastlog`.
- Busybox provides `last` (symlink); it errors when `/var/log/wtmp` is absent.
- Upstream Buildroot `openssh.mk` passes `--disable-utmp --disable-utmpx
  --disable-wtmp --disable-wtmpx --disable-lastlog`, so sshd never writes
  login records even if the files exist. `external.mk` already overrides other
  package `*_CONF_OPTS` the same way (e.g. RAUC).

`persist-var` only bind-mounts directories (`mkdir -p` then `mount --bind`), so
individual files cannot be listed in `persisted-var`.

## Goals / Non-Goals

**Goals:**
- `last` opens successfully after boot
- SSH logins as `obake` appear in `last`
- Those records survive reboot/slot switch via the persist partition
- Headless SSH verification catches regressions

**Non-Goals:**
- Enabling Linux-PAM or changing SSH auth policy
- Replacing Busybox `last` with util-linux `last`
- Full `who`/`w`/`lastb` UX polish beyond creating btmp/lastlog alongside wtmp
- Rotating or size-capping wtmp (low growth; revisit if needed)
- Backfilling history on hosts that never had wtmp before the OTA

## Decisions

### 1. Create wtmp family via overlay tmpfiles

Add `etc/tmpfiles.d/obake-wtmp.conf` (rootfs overlay):

```
f /var/log/wtmp 0664 root utmp -
f /var/log/btmp 0660 root utmp -
f /var/log/lastlog 0664 root utmp -
```

Matches Debian's conventions; `utmp` group already exists in the image.
`obake-persist-var` runs before `sysinit.target`, so when tmpfiles later create
these files under a persisted `/var/log`, they land on persist.

**Alternatives considered:** Patching systemd's `var.conf` in the package —
fragile across systemd bumps. Creating files only in `post-fakeroot` — useless
because `/var` is tmpfs at runtime.

### 2. Persist all of `/var/log`, not only the journal

Change `/etc/obake/persisted-var` from `log/journal` to `log`. One directory
bind covers journal, wtmp, btmp, and lastlog without teaching `persist-var` to
handle files.

**Migration:** Existing hosts already have `/persist/var/log/journal`. Binding
`/persist/var/log` → `/var/log` keeps that tree at the same relative path.
No data copy step.

**Alternatives considered:** Persist only a new `log/wtmp` path — impossible
with directory-only bind mounts unless we store a directory of accounting
files and symlink (more moving parts). Keep wtmp volatile — fails the
"history across reboot" goal for debugging.

### 3. Re-enable OpenSSH utmp/wtmp via `external.mk`

In `image/os/buildroot/external.mk`, filter out Buildroot's disable flags:

```make
OPENSSH_CONF_OPTS := $(filter-out \
	--disable-lastlog --disable-utmp --disable-utmpx \
	--disable-wtmp --disable-wtmpx,$(OPENSSH_CONF_OPTS))
```

Default OpenSSH configure then uses libc `WTMP_FILE` / `UTMP_FILE` paths
(`/var/log/wtmp`, `/var/run/utmp`). Forces an openssh rebuild on next image
build.

**Alternatives considered:** Full package fork under `package/openssh/` —
heavier than the established CONF_OPTS override pattern. Enabling linux-pam
solely for session logging — out of scope and larger surface.

### 4. Verify in `ssh-test.sh`

After SSH is up, force a PTY session (`ssh -tt`) and run something
equivalent to:

```sh
last 2>/dev/null | grep -q obake
```

OpenSSH only calls `record_login` when allocating a PTY, so a non-TTY
remote command never writes wtmp. Fail the harness if `last` errors or
omits `obake`. Fits the existing diagnostics block; no new QEMU scenario
required for within-boot recording.
Cross-reboot persistence remains covered by the host-config requirement and
can be spot-checked manually or folded into persist-test later if needed.

## Risks / Trade-offs

- [OpenSSH rebuild] → Filter flags only; no source patches. Rebuild cost is
  normal for an image change that touches sshd.
- [Persisting all of `/var/log`] → Could retain unexpected log files if future
  packages write there. Today the image is journal-centric; accept and document.
  Mitigate later with logrotate if growth appears.
- [Empty historical wtmp on first boot after OTA] → Expected; only new logins
  are recorded. No migration of fictional history.
- [Busybox `last` quirks] → Acceptable; the failure mode today is missing file,
  not applet quality.

## Migration Plan

1. Ship via normal slot image / OTA; no installer schema change.
2. On first boot of the new image, `persist-var` bind-mounts `/persist/var/log`
   (creating it if needed); existing journal data remains under `journal/`.
3. Rollback to an older slot: old image still bind-mounts only
   `log/journal`; leftover sibling files under `/persist/var/log/` are
   harmless.
