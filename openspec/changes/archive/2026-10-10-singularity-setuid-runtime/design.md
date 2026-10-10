# Design

## Context

See proposal.md for motivation. SingularityCE is built at
`image/os/buildroot/package/singularity/` with
`./mconfig --without-suid --without-libsubid`. Guest verify runs
`singularity run /opt/obake/verify.sif` as `obake` and only checks exit status,
so a FUSE mount counts as success. The shared host fragment already enables
loop, SquashFS, FUSE, and `USER_NS`. Buildroot installs via `DESTDIR` as a
non-root user; setuid bits need an explicit permissions table (or equivalent
post-fakeroot chmod/chown). `/etc` is an overlay on persist, so seeded
`subuid`/`subgid` and `singularity.conf` follow the usual overlay contract.

## Goals / Non-Goals

**Goals:**
- Default native SIF run path for `obake`: setuid helper + kernel SquashFS
- On-device `--fakeroot` usable by `obake` with image-seeded maps
- Guest verify proves kernel mount and fakeroot (not merely "container exits 0")
- Docs match the new privilege model

**Non-Goals:**
- Trimming `USER_NS` / FUSE from the kernel fragment
- Enabling libsubid / NSS remote subid sources
- Apptainer migration
- Changing shape `.def` files or guaranteeing offline `Bootstrap: docker` builds
- Performance microbenchmarks in CI (correctness of mount path is the bar)

## Decisions

### 1. Build Singularity with setuid; keep `--without-libsubid`

Drop `--without-suid` from `mconfig` so `starter-setuid` is produced. Keep
`--without-libsubid`: the appliance has no LDAP/NSS subid story, and
SingularityCE reads `/etc/subuid` and `/etc/subgid` directly without libsubid.

**Alternative considered:** keep non-setuid and optimize squashfuse_ll.
Rejected: operator chose performance/privilege tradeoff toward setuid + kernel
SquashFS.

### 2. Apply setuid mode via Buildroot package permissions

Declare `SINGULARITY_PERMISSIONS` for the setuid starter path (SingularityCE
installs it under `libexec/singularity/bin/`; confirm the exact installed name
`starter-suid` / `starter-setuid` against the 4.5.1 tree) as
`f 4755 0 0 - - - - -`. Rely on Buildroot's fakeroot image assembly rather than
hoping `make install DESTDIR=...` preserves the bit.

Also ensure `/etc/singularity/singularity.conf` is root:root and not
group/other-writable (setuid installs refuse a writable conf).

**Alternative considered:** chmod in `post-fakeroot.sh` only. Rejected as the
primary mechanism: package-scoped `*_PERMISSIONS` is the Buildroot idiom and
survives package reinstalls cleanly; post-fakeroot may still reinforce conf
ownership if needed.

### 3. Pin kernel SquashFS in singularity.conf

Ship (or patch at install) conf directives:

```
allow setuid = yes
allow kernel squashfs = yes
```

Defaults are already yes on stock SingularityCE; pin them so a future upstream
default flip cannot silently regress to FUSE.

### 4. Seed `/etc/subuid` and `/etc/subgid` for `obake` at image build

Add a fixed non-overlapping range (at least 65536 IDs) for `obake` in the
rootfs overlay or generate it in `post-fakeroot.sh` (e.g. invoke
`singularity config fakeroot --add obake` against `TARGET_DIR` if that works
under fakeroot, otherwise write the two files statically). Prefer a static,
reproducible mapping checked into the overlay so rebuilds do not drift.

**Alternative considered:** require operators to run `sudo singularity config
fakeroot --add obake` after install. Rejected: on-device fakeroot must work on
first boot without a manual step.

### 5. Guest verify: prove mount path + light fakeroot

Extend `check_singularity_run` (or replace it) so success requires evidence of
kernel SquashFS, for example:

- `singularity -d run …` log must not indicate squashfuse as the image mount,
  and/or
- while the container runs, observe the mount (debug/findmnt from a helper, or
  parse debug output for kernel squashfs / loop mount language used by 4.5.1)

Add `check_singularity_fakeroot` that runs
`singularity exec --fakeroot /opt/obake/verify.sif id -u` and expects `0`.
Keep using the existing VERIFY-only SIF fixture; do not pull `library://`.

**Alternative considered:** full `singularity build --fakeroot` of a tiny def
in guest CI. Deferred: heavier, network-sensitive with `Bootstrap: docker`;
`exec --fakeroot` proves maps + setuid fakeroot path.

### 6. Docs for operator-facing language; enable POSIX mqueue in fragment

Update user-guide / dev-qemu (and any os README lines) that claim non-setuid /
FUSE-only shapes. Keep `USER_NS` and FUSE in `linux-obake.fragment` (still
needed for fakeroot and fallbacks). Also set `CONFIG_POSIX_MQUEUE=y` there:
Singularity mounts an `mqueue` filesystem into the session under `--fakeroot`,
and some board base configs (notably `generic-x86_64`) omit it, which fails
with `can't mount mqueue …: no such device`.

## Risks / Trade-offs

- **[Risk] Setuid helper widens privilege surface** → Mitigation: intentional;
  single-tenant appliance; conf not user-writable; document the model.
- **[Risk] DESTDIR install leaves starter non-setuid → silent FUSE fallback** →
  Mitigation: `SINGULARITY_PERMISSIONS` + guest check that fails on FUSE path.
- **[Risk] Writable singularity.conf breaks setuid runs** → Mitigation: enforce
  ownership/mode in image assembly; verify refuses if conf is wrong.
- **[Risk] subuid range collides with future system UIDs** → Mitigation: pick a
  high fixed range (document in design/tasks) away from `obake`'s real uid.
- **[Trade-off] Fakeroot smoke ≠ full on-device shape rebuild** → Acceptable;
  docs note docker bootstrap / network still needed for current shape defs.
- **[Trade-off] FUSE remains installed** → Acceptable fallback; not the verified
  primary path.

## Migration Plan

- Rebuild board images; OTA delivers new runtime + `/etc` map/conf on the next
  signed slot (`/etc` overlay upper may need the new files if an older upper
  shadows them — prefer shipping defaults that apply when absent, or document
  that a clean `/etc` upper / re-merge may be needed for fakeroot on upgraded
  hosts that already customized `/etc`).
- No shape rebuild; existing SIFs keep working.
- Rollback: previous slot retains non-setuid runtime.

## Open Questions

None that block specs or task breakdown. Exact debug-log strings / findmnt
probe for "kernel SquashFS vs FUSE" can be settled against SingularityCE 4.5.1
during implementation.
