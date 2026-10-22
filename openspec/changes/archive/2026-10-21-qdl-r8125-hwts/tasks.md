# Tasks

## 1. Vendor source in the recipe

- [x] 1.1 Copy `r8125-9.018.00.tar.bz2` into
  `image/os/buildroot/package/r8125/` (source tarball is
  `/mnt/c/Users/g1988/Downloads/r8125-9.018.00.tar.bz2`). Track
  `os/buildroot/package/r8125/*.tar.bz2` from `image/.gitattributes` with
  Git LFS and stage the tarball so the commit stores an LFS pointer, not
  the bz2 bytes. Verify with `git check-attr filter` that the path is
  `lfs`, and with `git lfs ls-files` that the tarball is listed.

## 2. qdl kernel and firmware overlay

- [x] 2.1 In `image/os/buildroot/board/obake/qdl/linux.config`, replace
  `CONFIG_R8169=m` with `# CONFIG_R8169 is not set`. Keep
  `CONFIG_NET_VENDOR_REALTEK=y`, `CONFIG_PTP_1588_CLOCK=y`,
  `CONFIG_X86_INTEL_LPSS=y`, `CONFIG_INTEL_MEI=m`, `CONFIG_INTEL_MEI_ME=m`,
  and `CONFIG_HW_RANDOM_INTEL=y`. Verify with `rg` that this file does not
  set `CONFIG_R8169=m` or `CONFIG_IGC`, and that the platform Intel options
  above are still set.
- [x] 2.2 Delete `rtl8125a-3.fw`, `rtl8125b-2.fw`, `rtl8125d-1.fw`, and
  `LICENSE.r8169` from
  `image/os/buildroot/board/obake/qdl/rootfs-overlay/usr/lib/firmware/rtl_nic/`.
  Verify that directory no longer contains those files.

## 3. r8125 package

- [x] 3.1 Add `image/os/buildroot/package/r8125/Config.in` and `r8125.mk`.
  Pin version `9.018.00`, take the tarball from the recipe directory, and
  build `src/` with the kernel-module infra. Pass
  `ENABLE_PTP_SUPPORT=y`, `ENABLE_EEE=n`, `CONFIG_ASPM=n`,
  `CONFIG_SOC_LAN=n`, `ENABLE_DASH_SUPPORT=n`,
  `ENABLE_MULTIPLE_TX_QUEUE=n`, `ENABLE_RSS_SUPPORT=n`, and
  `ENABLE_USE_FIRMWARE_FILE=n`. Source the Config.in from
  `image/os/buildroot/Config.in`. Verify the makefile lists those eight
  flags and `external.mk`'s wildcard picks up `r8125.mk` (no new include
  line).
- [x] 3.2 Add a Buildroot patch under `image/os/buildroot/package/r8125/`
  that implements `ndo_hwtstamp_get` and `ndo_hwtstamp_set` by calling the
  vendor `rtl8125_get_tstamp` / `rtl8125_set_tstamp` helpers, and marks the
  config already copied to user space. Do not take `rtnl` in the shim.
  Verify the patch file is present and references both netdev ops.
- [x] 3.3 Select `BR2_PACKAGE_R8125=y` only in
  `image/os/buildroot/configs/obake-qdl.config`, and correct that file's
  header comment so it names RTL8125 / `r8125` rather than I226-V. Verify
  with `rg` that no other `image/os/buildroot/configs/*.config` selects
  `BR2_PACKAGE_R8125`.
- [x] 3.4 In `image/docs/user-guide.md`, name `r8125` (with hardware
  timestamping) as the `qdl` NIC driver, and state that a PTP capability
  check on that NIC is a board test, not a QEMU test. Verify with `rg`
  that the guide no longer calls the `qdl` NIC `r8169`.

## 4. Headless image check

- [x] 4.1 Build `qdl` headlessly with `image/os/buildroot/build.sh qdl` and
  verify the output linux `.config` has `# CONFIG_R8169 is not set`, still
  sets `CONFIG_PTP_1588_CLOCK`, `CONFIG_X86_INTEL_LPSS`, `CONFIG_INTEL_MEI`,
  and `CONFIG_HW_RANDOM_INTEL`, does not set `CONFIG_IGC`, the target
  rootfs contains `r8125.ko` and does not contain `r8169.ko` or
  `rtl8125b-2.fw`, and the module build linked with PTP support (the
  `r8125` objects include `r8125_ptp`). No shape version bump: this change
  does not touch a shape dependency.
- [x] 4.2 On a Quieter DL running the new image, run `ethtool -T` on an
  RTL8125 interface and `hwstamp_ctl -i <if> -r 1 -t 1` (via `ptp-tools` or
  the host). Verify the interface reports hardware transmit and receive
  timestamping, a PTP clock index, and that the two-step / PTPv2 event
  stamp request succeeds. This step needs the board; QEMU cannot do it.
