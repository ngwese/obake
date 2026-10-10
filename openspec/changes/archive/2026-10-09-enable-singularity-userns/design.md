# Design

## Context

See proposal.md for motivation. Singularity CE is built with
`--without-suid --without-libsubid` and mounts SIF images via FUSE. The shared
host fragment already enables loop, SquashFS, and FUSE, but no board kernel
enables `CONFIG_NAMESPACES` / `CONFIG_USER_NS`. Cgroup coverage is uneven
(`generic-aarch64` has several controllers; x86 boards mostly only
`CONFIG_CGROUPS`). Guest suites (`obake-verify.service` and related) run as
root today; only `ssh-test.sh` exercises the operator identity. `obake-update`
already re-execs via narrowly scoped sudo for `apply`/`rollback` and supports
`local_bundle=` in `/etc/obake/update.conf`, but guest RAUC checks call
`rauc install` directly.

## Goals / Non-Goals

**Goals:**
- Shared kernel fragment owns namespaces + common cgroup controllers for every
  board
- Keep Singularity non-setuid; fix the kernel side of that contract
- Strict guest verify as `User=obake` across functional / update / persist /
  health / integration guest services
- Catch missing userns with a local SIF run (no network registry)
- Route install/reject/rollback through `obake-update apply [path]`

**Non-Goals:**
- Enabling setuid Singularity / Apptainer migration
- Making `/etc` writable by the `obake` login user
- Shipping a full container orchestration stack or cgroup v1-only layout
- Network-dependent `library://` pulls inside guest verify
- Changing shape definition files

## Decisions

### 1. Kernel options live in `linux-obake.fragment`

Add to the shared host fragment (not per-board copies) at least:

```
CONFIG_NAMESPACES=y
CONFIG_UTS_NS=y
CONFIG_IPC_NS=y
CONFIG_USER_NS=y
CONFIG_PID_NS=y
CONFIG_NET_NS=y
CONFIG_CGROUPS=y
CONFIG_CGROUP_CPUACCT=y
CONFIG_CGROUP_DEVICE=y
CONFIG_CGROUP_FREEZER=y
CONFIG_CGROUP_PIDS=y
CONFIG_CGROUP_SCHED=y
CONFIG_CPUSETS=y
CONFIG_MEMCG=y
CONFIG_BLK_CGROUP=y
```

Also enable cgroup namespace when the Kconfig tree offers it
(`CONFIG_CGROUP_NS` / equivalent for the pinned kernel). Do **not** enable
`CONFIG_RT_GROUP_SCHED` (fights PREEMPT_RT). Prefer cgroup v2 defaults already
used by systemd on the image; do not force a cgroup-v1 hierarchy.

Board `linux.config` files may already set a subset; leave duplicates alone
unless a board explicitly disables an option the fragment needs — then remove
the disable so the fragment wins.

**Alternative considered:** rebuild Singularity with setuid. Rejected: reverses
the intentional non-setuid packaging and widens the privilege surface.

### 2. Keep `--without-suid`; no `/etc/subuid` requirement for the smoke test

The runtime stays as packaged. The smoke test runs a trivial read-only SIF as
`obake` (no fakeroot / no `--fakeroot`), so libsubid and subuid maps are not
required for the verification bar. Document that advanced fakeroot workflows
remain out of scope.

### 3. Guest units: `User=obake` + polkit poweroff

In `install-verify.sh`, set `User=obake` / `Group=obake` on
`obake-verify.service` and the other guest oneshot suites (update, persist,
health, integration). Replace `systemctl poweroff -f` with a polkit-allowed
`systemctl poweroff` (or `loginctl poweroff`) that works for `obake`. Keep
console logging via journal+console.

**Alternative considered:** keep a root wrapper that `su`s to `obake` per
check. Rejected: easy to regress back into root-only probes; strict unit
identity matches production SSH sessions more closely.

### 4. Optional bundle path on `obake-update apply`

Extend the client to:

```
obake-update apply [<bundle-path>]
```

When `<bundle-path>` is present, install that file (after the usual snapshot /
pending health write) with the same `rauc install` path used for
`local_bundle=`. When absent, keep today's USB → HTTPS → USB-fallback order
(and configured `local_bundle=`). Document the path form in `os-update.md`.

This lets VERIFY images reject a tampered copy under `/home` or `/opt` without
editing `/etc` as `obake`.

### 5. Embedded tiny SIF for verify images only

When `VERIFY=1`, `install-verify.sh` installs a small SIF (for example a
minimal busybox/`true` image built once in the verify/build tooling, or a
checked-in fixture under `image/verification/fixtures/`) at a fixed path such
as `/opt/obake/verify.sif`, readable by `obake`. Guest check runs
`singularity run /opt/obake/verify.sif` as `obake`. Production (non-VERIFY)
images do not ship the fixture.

### 6. Rewrite contract / RAUC / root-lock probes for login-user reality

| Check | New probe as `obake` |
|-------|----------------------|
| root readonly | write under `/` fails |
| home writable | write under `/home/obake` succeeds |
| `/etc` denied | write under `/etc` fails |
| etc overlay | `findmnt` fstype `overlay` (+ overlay marker readable) |
| var contract | tmpfs + persisted bind mountpoints; no root-owned write probe |
| root locked | empty-password `su` fails (not `/etc/shadow` read) |
| update sudo | `sudo -n obake-update status` ok; shell sudo denied |
| rauc install/reject/rollback | `obake-update apply` / `apply <bad>` / `rollback` |
| singularity | version + local SIF run |

Persist guest test: stop writing `/etc` and root-owned `/var/lib/systemd`
markers as proof of login-user behavior. Keep topology assertions
(`findmnt` / mountpoints) and user-partition markers under `/home`. Slot-switch
activation goes through `obake-update rollback` or an already-granted path —
not raw `rauc status mark-active` as root. If a multi-boot persist suite still
needs a system-written `/etc` marker, seed it at image install time rather than
writing it from the guest check process.

### 7. Docs: clarify login-user vs system-writable contract

Update `host-config.md` so the directory-contract section matches the spec:
named mounts remain system-writable; SSH `obake` writes `/home` and cannot
write `/etc`. Note Singularity userns dependency briefly in the user guide /
dev-qemu smoke path.

## Risks / Trade-offs

- **[Risk] USER_NS widens kernel attack surface** → Mitigation: intentional for
  non-setuid containers; no setuid helper; keep image otherwise minimal.
- **[Risk] RT + cgroup controllers interact badly** → Mitigation: omit
  `RT_GROUP_SCHED`; verify under QEMU with the realtime kernel before hardware.
- **[Risk] Polkit poweroff from a non-SSH session fails** → Mitigation: guest
  unit may need `PAMName=` / bus session tweaks, or a tiny wrapper still
  confined to poweroff; validate in `verify-test.sh` early.
- **[Risk] Tiny SIF build adds verify-image complexity** → Mitigation: prefer a
  small checked-in fixture over a network pull; build only when regenerating
  the fixture.
- **[Trade-off] Persist suite no longer proves `/etc` write-as-user** →
  Acceptable under option 1; overlay topology + install-seeded markers cover
  system persistence.

## Migration Plan

- Rebuild board images; OTA delivers the new kernel + client/docs on the next
  signed slot.
- No persist migration; no shape rebuild required.
- Rollback: previous slot retains the old kernel (Singularity still broken
  there); revert the fragment/verify/client commits for a new build if needed.

## Open Questions

None that affect specs or task breakdown. Exact SIF fixture contents and
polkit session details can be settled during implementation against
`verify-test.sh`.
