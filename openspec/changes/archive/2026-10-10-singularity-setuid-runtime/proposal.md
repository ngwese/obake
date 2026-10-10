# Proposal

## Why

Shapes run as `obake` through a non-setuid SingularityCE build that mounts SIF
images over FUSE (`squashfuse`). That path adds userspace I/O overhead on the
container rootfs and fights the project's bias toward low-overhead access to
host hardware. The appliance should prefer kernel SquashFS mounts via a setuid
helper, and should also support on-device `singularity build --fakeroot` so
operators can rebuild shapes on the box.

## What Changes

- Rebuild SingularityCE **with** the setuid helper (drop `--without-suid`); keep
  `--without-libsubid` and seed `/etc/subuid` / `/etc/subgid` for `obake` so
  setuid fakeroot reads local maps.
- Ensure Buildroot installs `starter-setuid` as root-owned mode `4755`, and that
  `singularity.conf` is root-owned, not user-writable, with `allow setuid = yes`
  and `allow kernel squashfs = yes`.
- Keep kernel user namespaces and FUSE enabled (needed for fakeroot and as
  fallback); do not trim the host kernel fragment in this change.
- Update guest verification so a local SIF run as `obake` proves the **kernel
  SquashFS** path (not FUSE), and add a light `--fakeroot` smoke check.
- Update operator/docs language that currently claims "no setuid helper" /
  FUSE-only shapes.
- **BREAKING** (runtime privilege model): newly built images ship a setuid
  Singularity helper. Persist layout and login credentials are unchanged; hosts
  pick this up on the next OTA slot. Shape `.def` files are unchanged.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `os-image-build`: require a setuid SingularityCE runtime that mounts local
  SIFs via kernel SquashFS for `obake`, and provision fakeroot subordinate ID
  maps so on-device `--fakeroot` works; reframe namespace requirements as
  supporting setuid fakeroot (and fallbacks), not as a non-setuid-only contract.
- `image-verification`: require guest checks that prove kernel SquashFS for the
  local SIF run and that `--fakeroot` works as `obake`.

## Impact

- `image/os/buildroot/package/singularity/` (`singularity.mk`, `Config.in`,
  permissions table for `starter-setuid`).
- Image provisioning of `/etc/subuid` / `/etc/subgid` and `singularity.conf`
  (rootfs overlay and/or post-build / package install hooks).
- `image/verification/guest/obake-verify.sh` and any VERIFY fixture notes.
- Docs: `image/docs/user-guide.md`, `image/docs/dev-qemu.md`, and related
  host-config / os README mentions of non-setuid / FUSE.
- OTA: runtime + `/etc` map files ship in a new signed image; no shape rebuild
  required for existing SIFs.
- Out of scope: kernel fragment cleanup, Apptainer migration, changing shape
  definition files, making every `Bootstrap: docker` shape offline-buildable.
