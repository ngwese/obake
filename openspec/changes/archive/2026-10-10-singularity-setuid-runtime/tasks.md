# Tasks

## 1. SingularityCE setuid packaging

- [x] 1.1 In `image/os/buildroot/package/singularity/singularity.mk`, drop
      `--without-suid` from `mconfig` (keep `--without-libsubid` and the rest)
      and update `Config.in` help text that still describes a non-setuid build.
      Verify: the package recipe no longer passes `--without-suid`, and
      `Config.in` states setuid is enabled.
- [x] 1.2 Add `SINGULARITY_PERMISSIONS` (or equivalent) so the installed
      `starter-suid` / `starter-setuid` path under
      `libexec/singularity/bin/` is `f 4755 0 0` in the assembled rootfs.
      Confirm the exact filename against SingularityCE 4.5.1. Verify: a built
      target rootfs (or image listing) shows that binary mode `4755` owned by
      root.
- [x] 1.3 Ship `/etc/singularity/singularity.conf` with `allow setuid = yes`
      and `allow kernel squashfs = yes`, root-owned and not writable by
      non-root (overlay and/or package install + `post-fakeroot.sh` as needed).
      Verify: conf on a built image matches those directives and permissions.

## 2. Fakeroot maps for obake

- [x] 2.1 Seed reproducible `/etc/subuid` and `/etc/subgid` entries for
      `obake` with at least 65536 IDs each (rootfs overlay under
      `image/os/buildroot/board/obake/` or `post-fakeroot.sh`), using a fixed
      high range that does not collide with the real `obake` uid/gid. Verify:
      both files exist in a built rootfs with an `obake:` line and count
      >= 65536.
- [x] 2.2 Document in `image/docs/user-guide.md` (and `dev-qemu.md` where it
      still says non-setuid/USER_NS-only) that shapes run via the setuid helper
      with kernel SquashFS, that `--fakeroot` is supported for on-device builds,
      and that current shape `.def` files may still need network/docker
      bootstrap beyond fakeroot. Verify: docs no longer claim "no setuid
      helper" / FUSE-only as the production path.

## 3. Guest verification

- [x] 3.1 Update `image/verification/guest/obake-verify.sh`
      `check_singularity_run` so success requires kernel SquashFS for the
      embedded `/opt/obake/verify.sif` run as `obake` (fail if squashfuse/FUSE
      is the image mount path). Verify: the check script encodes an explicit
      kernel-vs-FUSE assertion (debug log and/or mount probe).
- [x] 3.2 Add a guest check that runs
      `singularity exec --fakeroot /opt/obake/verify.sif id -u` as `obake` and
      expects `0`; wire it into the functional suite next to the existing
      Singularity checks. Verify: the new check name appears in
      `obake-verify.sh` and fails when subuid maps are missing.
- [x] 3.3 Rebuild a VERIFY image for at least one board profile and run the
      headless QEMU guest functional suite; confirm Singularity version, mount,
      run-via-kernel-SquashFS, and fakeroot checks pass as `obake`. Verify:
      guest suite reports pass for those checks without elevated host cleanup.
