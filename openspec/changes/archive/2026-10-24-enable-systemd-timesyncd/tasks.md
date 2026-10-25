# Tasks

## 1. Enable timesyncd in the image

- [x] 1.1 Add `BR2_PACKAGE_SYSTEMD_TIMESYNCD=y` to
      `image/os/buildroot/configs/obake_base.config` next to the other
      systemd package options. Verify: the defconfig line is present and a
      composed board defconfig still selects timesyncd.
- [x] 1.2 Add overlay wants symlink
      `image/os/buildroot/board/obake/rootfs-overlay/etc/systemd/system/sysinit.target.wants/systemd-timesyncd.service`
      → `/usr/lib/systemd/system/systemd-timesyncd.service`. Verify: the
      symlink resolves to that path (same pattern as networkd/resolved).
- [x] 1.3 Document network time sync in `image/docs/host-config.md` under
      Network and tooling: timesyncd enabled by default; boot does not wait
      on NTP; operators enabling `phc2sys` should stop/disable timesyncd.
      Verify: the doc mentions those three points.

## 2. Headless verification

- [x] 2.1 Add `check_timesyncd` in
      `image/verification/guest/obake-verify.sh` that requires
      `systemctl is-enabled` and `is-active` for `systemd-timesyncd.service`
      (do not require NTPSynchronized); wire it into the `run` list. Verify:
      the function exists and is invoked from the main checklist.
- [x] 2.2 Rebuild (or reinstall/OTA) an image that includes the change and
      run the guest verification suite. Verify: `timesyncd` check passes;
      `systemctl is-enabled systemd-timesyncd` is enabled on the booted host.
