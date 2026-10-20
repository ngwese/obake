# Tasks

## 1. Enable PAM in the base image

- [x] 1.1 Add `BR2_PACKAGE_LINUX_PAM=y` to
  `image/os/buildroot/configs/obake_base.config`, and verify the composed
  defconfig (or `make` config dump for the package) selects linux-pam
- [x] 1.2 Rebuild or refresh an OS image that includes the new base config,
  and verify the target rootfs contains Linux-PAM libraries and
  `/etc/pam.d/sshd`, and that `sshd_config` enables `UsePAM yes`
- [x] 1.3 On that image, verify systemd is built with PAM (for example
  `systemctl --version` shows `+PAM`, or equivalent feature string)

## 2. PAM session stacks and limits.d

- [x] 2.1 Inspect Buildroot-installed `/etc/pam.d/sshd` (and `login` if
  present). If `pam_limits.so` or `pam_systemd.so` is missing from the
  session stack, add an overlay under
  `image/os/buildroot/board/obake/rootfs-overlay/etc/pam.d/` (or a minimal
  `post-build.sh` tweak) so both are present, and verify the installed
  `sshd` PAM file lists them
- [x] 2.2 Confirm
  `image/os/buildroot/board/obake/rootfs-overlay/etc/security/limits.d/obake-audio.conf`
  still ships `@audio` rtprio 95 and unlimited memlock on the image, and
  verify those lines appear under `/etc/security/limits.d/` in the rootfs

## 3. Remove no-PAM workarounds

- [x] 3.1 Delete
  `image/os/buildroot/board/obake/rootfs-overlay/etc/profile.d/obake-user-bus.sh`,
  and verify it is absent from the built rootfs
- [x] 3.2 Delete
  `image/os/buildroot/board/obake/rootfs-overlay/usr/lib/systemd/system/user@.service.d/20-obake-xdg-runtime.conf`
  (and the empty drop-in directory if nothing else remains), and verify
  that path is absent from the built rootfs while
  `obake-linger.service` still starts `user-runtime-dir@` and `user@`

## 4. Docs and headless verification

- [x] 4.1 Update `image/os/README.md` (and `image/host/SETUP.md` if it still
  describes the no-PAM user-bus path) to document PAM, `pam_limits` /
  `limits.d` for login shells, and removal of the profile.d /
  `user@.service.d` workarounds; verify those docs no longer claim
  "without PAM" / `obake-user-bus.sh`
- [x] 4.2 Extend `image/verification/qemu/ssh-test.sh` so the SSH login
  shell asserts `ulimit -r` is 95 and `ulimit -l` is unlimited, in addition
  to the existing `systemctl --user status` check, and verify the script
  contains those assertions
- [x] 4.3 Run the headless QEMU SSH verification path that exercises
  `ssh-test.sh` on an image built with this change, and verify it passes
  user-bus reachability and the new rlimit checks; also confirm guest
  PipeWire/WirePlumber unit `LimitRTPRIO` checks still pass
