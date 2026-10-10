# Proposal

## Why

On a booted host, Singularity prints
`WARNING: 'nodev' mount option set on /tmp, it could be a source of failure
during build process` and can then fail, for on-device `singularity build` and
for runs that first convert an image (for example `docker://` or `library://`).
systemd 258, as shipped by the pinned Buildroot, mounts `/tmp` as tmpfs with
`nodev`. SingularityCE 4.5.1 uses that directory as its default build sandbox
and needs to create device nodes there. A local SIF run does not take this
path, so only some containers show the warning.

## What Changes

- Mount `/tmp` as a volatile tmpfs that stays `nosuid` and is not `nodev` or
  `noexec`, so the default Singularity temporary directory can hold device
  nodes and executed build scripts.
- Keep `/tmp` discarded across boot. Do not persist it, and do not move
  Singularity's temporary directory onto `/home` or `/var`.
- Document `/tmp` in the directory contract as that volatile mount.
- Fail headless verification if `/tmp` is `nodev` or `noexec`, or if a local
  `singularity build --fakeroot` emits the `nodev` warning.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `host-config`: name `/tmp` as a volatile tmpfs that `obake` can write, with
  device nodes allowed and setuid execution not allowed.
- `os-image-build`: on-device Singularity builds and image conversions that
  use the default temporary directory must not fail because `/tmp` is `nodev`.
- `image-verification`: after boot, fail if `/tmp` carries `nodev` or `noexec`,
  or if a local fakeroot build warns about `nodev` on `/tmp`.

## Impact

- Rootfs overlay drop-in for systemd's `tmp.mount` (the unit Buildroot's
  systemd 258 enables with `nodev`). No change to shape definition files.
- `etc/obake/directory-contract` and `image/docs/host-config.md`.
- `image/verification/guest/obake-verify.sh`, using the existing embedded
  verify SIF so the check stays offline.
- Persistence, OTA slot layout, and cross-host shape portability are
  unchanged. `/tmp` remains volatile; built SIFs the user saves under
  `/home` still persist.
