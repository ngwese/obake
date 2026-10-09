# Tasks

## 1. Rename INSTALLER_CONSOLE to CONSOLE

- [x] 1.1 Rename `INSTALLER_CONSOLE` to `CONSOLE` in
      `image/os/buildroot/profiles/*/*.conf`,
      `image/os/buildroot/build.sh` (profile allow-list and log line),
      `image/build/build-env/run.sh`, and
      `image/os/buildroot/board/obake/make-installer.sh`; verify
      `rg INSTALLER_CONSOLE image/` returns no matches outside archive/docs
      still pending update
- [x] 1.2 Update `image/installer/README.md` and `image/docs/user-guide.md` to
      document `CONSOLE` (and note the rename); verify the documented
      `CONSOLE=local` / `CONSOLE=serial` examples match `make-installer.sh`

## 2. Shared console for installer and OS UKIs

- [x] 2.1 Teach `image/os/buildroot/board/obake/post-image.sh` to build OS
      UKI cmdlines from `CONSOLE` using the same serial/local mapping as
      `make-installer.sh` (including `rk35xx` → `ttyS2`); verify a
      `CONSOLE=local` build embeds `console=tty0` in `obake-a.efi` /
      `obake-b.efi` and a `CONSOLE=serial` x86_64 build embeds
      `console=ttyS0`
- [x] 2.2 Align the installed image getty with `CONSOLE` (local → VT getty,
      serial → board serial getty, preserving `rk35xx` `ttyS2`); verify the
      staged rootfs enables the matching getty unit for each selection
- [x] 2.3 Build `qdl` with the `default` profile and verify both
      `installer-local.img` and the OS UKIs use the local console
      (`console=tty0`), matching the profile console requirement

## 3. Post-install shutdown wait

- [x] 3.1 In `image/os/buildroot/board/obake/make-installer.sh`, after
      `install.sh` exits, prompt for confirmation when
      `INSTALLER_TARGET_DISK=interactive`, otherwise sleep
      `INSTALLER_POWEROFF_DELAY` seconds (default 10) before `poweroff`;
      verify the generated `obake-installer.service` contains the prompt path
      for interactive builds and a `sleep` (or equivalent) for
      non-interactive builds with the default delay
- [x] 3.2 Accept build-time `INSTALLER_POWEROFF_DELAY` (including `0`),
      forward it through `image/build/build-env/run.sh`, and document it in
      `image/installer/README.md`; verify `INSTALLER_POWEROFF_DELAY=0`
      produces a unit that powers off without sleeping
- [x] 3.3 Update QEMU verification scripts under
      `image/verification/qemu/` that build installers
      (`install-test.sh`, `install-interactive-test.sh`, `reinstall-test.sh`,
      `integration-test.sh`, `dev-vm.sh`) to pass `CONSOLE` (renamed) and
      `INSTALLER_POWEROFF_DELAY=0`; verify
      `image/verification/qemu/install-test.sh` still completes headlessly
      and observes `OBKA_INSTALL_DONE`
