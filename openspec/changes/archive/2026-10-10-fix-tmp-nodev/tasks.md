# Tasks

## 1. /tmp tmpfs without nodev

- [x] 1.1 Add
      `image/os/buildroot/board/obake/rootfs-overlay/usr/lib/systemd/system/tmp.mount.d/obake.conf`
      replacing `tmp.mount` `Options=` with
      `mode=1777,strictatime,nosuid,size=50%%,nr_inodes=1m,x-systemd.graceful-option=usrquota`
      (literal `size=50%` after specifier expansion; no `nodev`, no
      `noexec`). Verify: the drop-in is the only change to that unit, and
      it does not mention `nodev` or `noexec`.
- [x] 1.2 Name `/tmp` as a volatile, non-persisted tmpfs in
      `image/os/buildroot/board/obake/rootfs-overlay/etc/obake/directory-contract`,
      `image/docs/host-config.md`, and the filesystem table in
      `image/docs/user-guide.md`. Do not add `/tmp` to
      `etc/obake/persisted-var`. Verify: all three texts call `/tmp`
      volatile and discarded; `persisted-var` has no `/tmp` entry.
- [x] 1.3 Extend `image/verification/guest/obake-persist-test.sh` so boot 1
      writes a marker under `/tmp` and later boots fail if that marker is
      still present. Verify: the script writes the marker only in phase 1
      and checks it is absent after reboot and after slot switch.
- [x] 1.4 Extend `image/verification/guest/obake-verify.sh` with a check,
      run as `obake`, that `findmnt` shows `/tmp` is tmpfs with `nosuid` and
      without `nodev` or `noexec`, and that `singularity build --fakeroot`
      from `/opt/obake/verify.sif` (`Bootstrap: localimage`, definition and
      output SIF under `/tmp`, no temporary-directory override) completes
      without the warning that `nodev` is set on `/tmp`. Verify: the new
      check is invoked from the script's check list; a headless QEMU guest
      verify run passes it.
