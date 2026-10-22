# Tasks

## 1. Package selection and source pin

- [x] 1.1 Select `BR2_PACKAGE_LINUXPTP=y` in
  `image/os/buildroot/configs/obake-qdl.config` only. Verify
  `image/os/buildroot/configs/obake_base.config` and the other
  `obake-*.config` fragments do not set it, and verify
  `shapes/ptp-tools/build-args.txt` still pins linuxptp 4.4 with no
  shape version bump.

- [x] 1.2 Add `linuxptp-4.4.tgz` under `image/os/buildroot/vendor/dl`
  (the existing Git LFS rule for `vendor/dl/**` covers it). Verify its
  sha256 matches `linuxptp.hash` in Buildroot 2026.02.3.

## 2. Units, device node, and configuration

- [x] 2.1 Add `ptp4l.service` and `phc2sys.service` under
  `image/os/buildroot/board/obake/qdl/rootfs-overlay/usr/lib/systemd/system/`.
  Verify both set `User=obake`, limit ambient and bounding capabilities
  as in `design.md`, are not wanted by a symlink in the overlay, and
  that `ptp4l.service` does not want `phc2sys.service`.

- [x] 2.2 Add a udev rule under
  `image/os/buildroot/board/obake/qdl/rootfs-overlay/etc/udev/rules.d/`
  that sets `KERNEL=="ptp[0-9]*"` to owner and group `obake`, mode
  `0660`. Verify `image/host/udev/` is unchanged.

- [x] 2.3 Add the AES67 client template and `phc2sys` options under
  `image/os/buildroot/board/obake/qdl/rootfs-overlay/usr/share/obake/linuxptp/`,
  plus a tmpfiles snippet that copies each into
  `/home/obake/.config/linuxptp/` only when missing. Verify the
  interface placeholder is `REPLACE_WITH_INTERFACE`, the client profile
  matches `design.md`, and the `phc2sys` options contain no step flag.

- [x] 2.4 Replace `/etc/linuxptp.cfg` from the `qdl` overlay with a
  comment pointing at `/home/obake/.config/linuxptp/`. Verify the
  packaged `eth0` client is not the file that remains.

- [x] 2.5 Document enabling the units and editing the home configuration
  in `image/docs/user-guide.md`. Verify the commands name `ptp4l` and
  `phc2sys` and tell the operator to replace
  `REPLACE_WITH_INTERFACE` before enabling.

## 3. Headless image check

- [x] 3.1 Build the `qdl` rootfs and verify it contains the nine linuxptp
  commands, the replacement units, no `multi-user.target.wants` symlink
  for either service, the udev rule, the templates, and the tmpfiles
  snippet. Verify a non-`qdl` Buildroot config still has
  `BR2_PACKAGE_LINUXPTP` unset.
