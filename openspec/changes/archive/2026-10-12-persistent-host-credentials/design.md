# Design

## Context

See proposal.md for motivation. Current behavior that shapes the approach:

- `image/build/build.sh` runs the Buildroot build in a container that mounts
  only the repository at `/work` and the cache volume at `/cache`
  (`image/build/build-env/run.sh`); a key path outside the repository is not
  visible inside it. `build.sh` also rewrites credential paths with
  `realpath -m`, a GNU extension that BSD `realpath` rejects on macOS.
- `post-build.sh` seeds a build-time key into rootfs
  `/etc/ssh/authorized_keys.d/obake`; `make-installer.sh` embeds it at
  `/opt/obake/authorized_key.pub`; `install.sh` copies it to the persist
  `/etc` overlay. `post-fakeroot.sh` hashes `OBKA_PASSWORD` into the rootfs
  shadow.
- `sshd_config.d/obake.conf` reads `.ssh/authorized_keys` and
  `/etc/ssh/authorized_keys.d/%u` (additive). `/home` is the obake-owned user
  partition; `/etc` is an overlay whose upper layer lives on `/persist`, shared
  across slots and never replaced by updates.
- The image builds busybox with `CONFIG_FEATURE_SUID=y` and `CONFIG_PASSWD=y`,
  so a setuid `passwd` should exist on the target.

## Goals / Non-Goals

**Goals:**
- Build-time key input works from any host path on macOS and Linux, failing
  loudly on an unreadable path.
- Credentials behave as seeds: applied once, never clobbering user-set values.
- The `obake` user manages its own key and password without elevated
  privileges; changes persist across updates and slot switches.
- No per-boot seeding service.

**Non-Goals:**
- Supplying a key interactively at install time (documented as a future option).
- Rotating a key seed through an OTA (the user partition is not updated).
- Encrypting credentials or changing the root-lock policy.

## Decisions

### 1. Stage the key into the build context instead of trusting the host path

`build.sh` copies the file named by `OBKA_SSH_AUTHORIZED_KEY` into a gitignored
build scratch directory (`image/_build/seeds/`) and rewrites the variable to
the `/work/...` path before launching the container. Rationale: it makes any
host location work and keeps the container mount contract unchanged.

Alternatives: bind-mount the key's parent directory into the container (leaks
arbitrary host paths and needs per-file mounts); pass the key contents in the
environment (awkward for multi-line input and harder to validate). Staging a
copy is simplest and also lets the build validate the file before the container
starts.

### 2. Portable absolute-path resolution

Replace `realpath -m` with a helper that resolves existing paths with plain
`realpath` and falls back to `cd "$(dirname)" && pwd -P`. Rationale: BSD
`realpath` lacks `-m`; the credential inputs must exist, so `-m`'s
non-existent-path behavior is not needed. `LAYOUT` already exists at the point
of use.

### 3. Key seed lives in the user's home, not a root-owned path

The installer writes the embedded seed to `~/.ssh/authorized_keys` on the user
partition, owned by `obake` (0700 `.ssh`, 0600 key), only when the user has no
key. The build stops seeding `/etc/ssh/authorized_keys.d/obake`. Rationale:
`/home` is obake-owned and persists across updates and user-preserving
reinstalls, so the seed is replaceable without sudo and cannot be re-imposed by
an update. The `AuthorizedKeysFile` path for `/etc/ssh/authorized_keys.d/%u` is
kept only as an operator provisioning fallback and is left empty by default.

Alternatives: keep the key under `/etc` and let the user shadow it — impossible
without sudo and additive, so replacing would not retire the seed; a boot unit
that copies a seed into the home — rejected to keep boot minimal.

### 4. Provisioning-time seeding only; no boot service

The key seed is written to the user partition whenever a target disk is
provisioned, and never by a boot service:

- The installer (`install.sh`) seeds while it already mounts the user partition
  (`seed_user`), deriving the `obake` uid/gid from its own passwd database (the
  installer rootfs is a copy of the target rootfs).
- Disk-image assembly (`os/scripts/make-disk.sh`, used by `MAKE_DISK=1` and the
  installer product) seeds the user-partition image it writes, so a flashed
  disk and the verification harnesses carry the same seed. Because it assembles
  the filesystem without mounting, it populates the user image with
  `mke2fs -d` and sets `obake` ownership under `fakeroot` (both present in the
  build environment).

Both paths apply the key only when the user has no key already
(`PRESERVE_USER` plus an existing file leaves it untouched). Rationale: meets
"seed unless the user set it" without adding a unit to the boot path.

### 5. Password stays a rootfs seed; persistence comes from overlay copy-up

`post-fakeroot.sh` keeps hashing `OBKA_PASSWORD` into the rootfs shadow. When
the user changes the password, the `/etc` overlay copies shadow up to
`/persist/etc-overlay/upper`, which then shadows later image seeds; when the
user has not changed it, a new slot image's seed shows through. Rationale: the
overlay already implements "seed unless the user set it," so no new mechanism
is required. Accepted trade-off under 6.

### 6. Password self-service uses the setuid busybox `passwd`, with a known limit

The image ships busybox with `FEATURE_SUID=y` and `PASSWD=y`, and Buildroot
installs `/usr/bin/passwd` setuid, so `obake` changes its own password with no
sudo **when the account already has a password**. busybox `passwd` requires the
old password whenever the shadow field is non-empty, so on a key-only image
(the default `*`) setting a first password needs elevation. Emptying the field
would let busybox `login` authenticate with no password and, because `obake`
has full sudo, hand out passwordless root from the serial console — so the `*`
lock stays.

Alternative considered: a small setuid helper to allow the first-password case.
Rejected as an accepted trade-off. Operators set a first password with
`sudo passwd obake` (no new privilege — `obake` already has full sudo) or seed
`OBKA_PASSWORD` at build time; `passwd` then works unprivileged for changes.
The root account stays locked.

## Risks / Trade-offs

- **[Shadow copy-up pins the whole file]** → A host where the user changed the
  password stops receiving image-side `/etc/shadow` changes (new system users,
  root-lock revisions). Accept as a documented trade-off; revisit only if
  updates begin to change shadow. No mitigation is implemented now.
- **[OTA cannot deliver a new key seed]** → `/home` is not updated, so key
  seeding is install-only. Document it; key rotation can be a future item.
- **[Seeding needs the user partition mounted at install]** → If the mount
  fails, no key is seeded and the host may be unreachable. Fail loudly and
  leave `OBKA_INSTALL_EXIT` nonzero.
- **[Ownership in an unmounted user image]** → `make-disk.sh` sets `obake`
  ownership with `fakeroot` + `mke2fs -d`; if `fakeroot` is unavailable the
  assembly fails loudly rather than producing a wrongly-owned key.
- **[uid/gid drift]** → Deriving `obake`'s uid/gid from the passwd database
  avoids hardcoding; a missing `obake` entry fails the install.
- **[Staged key in the repo tree]** → The scratch directory is gitignored and
  holds only a public key.
- **[Setuid `passwd` as a privilege boundary]** → Standard Unix behavior; root
  remains locked and the binary is the distribution's own.
- **[Existing hosts]** → Prior installs keep `/etc/ssh/authorized_keys.d/obake`,
  which sshd still honors; no forced migration. Operators may remove it after
  confirming a home key.

## Migration Plan

New builds and installs use the new paths; no data migration is required.
Rollback is reverting the scripts: existing home keys and overlay shadow remain
valid, and the retained `/etc/ssh/authorized_keys.d/%u` path keeps any prior
seed working.

## Open Questions

None blocking. Interactive install-time key entry and OTA key rotation are
explicit non-goals for this change.
