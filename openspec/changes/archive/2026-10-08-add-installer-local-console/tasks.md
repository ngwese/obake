# Tasks

## 1. qdl framebuffer console

- [x] 1.1 Add the EFI framebuffer text console to
      `image/os/buildroot/board/obake/qdl/linux.config` (`CONFIG_DRM=y`,
      `CONFIG_DRM_SIMPLEDRM=y`, `CONFIG_SYSFB=y`, `CONFIG_SYSFB_SIMPLEFB=y`,
      `CONFIG_DRM_FBDEV_EMULATION=y`, `CONFIG_FRAMEBUFFER_CONSOLE=y`) and build
      `image/os/buildroot/build.sh qdl`; verify the built kernel `.config` has
      those symbols `=y`, keeps `CONFIG_PREEMPT_RT=y`, and has
      `CONFIG_DRM_I915` unset

## 2. Installer console selection

- [x] 2.1 Implement `INSTALLER_CONSOLE=serial|local` in
      `image/os/buildroot/board/obake/make-installer.sh` (default `serial`;
      `local` runs the service on `tty1`, adds `console=tty0` to the installer
      UKI command line, and masks `getty@tty1.service`; reject an unrecognized
      value) and emit the medium as `installer-<console>.img`; verify both
      values: `installer-serial.img` keeps `ttyS0`, `installer-local.img`
      targets `tty1` and contains `console=tty0`
- [x] 2.2 Forward `INSTALLER_CONSOLE` through
      `image/verification/build-env/run.sh` and verify a containerized
      `INSTALLER_CONSOLE=local` build produces `installer-local.img`
- [x] 2.3 Document `INSTALLER_CONSOLE` and the bare-metal HDMI + USB keyboard
      flow in `image/installer/README.md`; verify the documented build command
      runs as written
- [x] 2.4 Update the installer harnesses and docs that reference the medium
      (`image/verification/qemu/{install-test,install-interactive-test,reinstall-test,integration-test,dev-vm}.sh`
      and `image/docs/dev-qemu.md`) to use `installer-serial.img` and to build
      the serial medium when it is absent; verify a local-only build is not
      reused by the serial harnesses

## 3. Integration verification

- [x] 3.1 Build `qdl` with `MAKE_INSTALLER=1 INSTALLER_CONSOLE=local` and
      assert the bundled installer UKI command line contains `console=tty0` and
      the installer service targets `tty1`
- [x] 3.2 Run the serial-default QEMU installer harnesses
      (`image/verification/qemu/install-test.sh` and
      `install-interactive-test.sh`) on `generic-x86_64` (the `qdl` kernel has
      no virtio drivers) and confirm they still pass unchanged
- [x] 3.3 Hardware smoke test: boot the `local` installer on the MeLE Quieter
      DL with an HDMI monitor and USB keyboard, complete disk selection and
      confirmation, and observe `OBKA_INSTALL_DONE`
