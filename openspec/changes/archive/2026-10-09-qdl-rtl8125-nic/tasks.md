# Tasks

## 1. qdl kernel Ethernet drivers

- [x] 1.1 In `image/os/buildroot/board/obake/qdl/linux.config`, set
  `# CONFIG_NET_VENDOR_INTEL is not set`, remove `CONFIG_IGC`, and enable
  `CONFIG_NET_VENDOR_REALTEK=y` and `CONFIG_R8169=m`. Update the file's
  header comment so it names RTL8125 instead of I226. Leave
  `CONFIG_X86_INTEL_LPSS`, `CONFIG_INTEL_MEI`, `CONFIG_INTEL_MEI_ME`, and
  `CONFIG_HW_RANDOM_INTEL` unchanged. Verify with `rg` that `IGC` is absent,
  Intel Ethernet is explicitly disabled, and `CONFIG_R8169=m` is present.
- [x] 1.2 In `openspec/changes/add-qdl-board/specs/os-image-build/spec.md`,
  change the `qdl` kernel-hardware requirement so it no longer requires the
  Intel IGC driver. Keep the MIDI 2.0 text. Verify with `rg` that this
  requirement no longer says the built config enables IGC.
- [x] 1.3 In `image/docs/user-guide.md`, replace the `qdl` hardware note that
  names I226/IGC Ethernet with RTL8125 via `r8169`. Verify with `rg` that
  the user guide no longer calls the `qdl` NIC an I226.

## 2. RTL8125 PHY firmware

- [x] 2.1 Copy `rtl8125a-3.fw`, `rtl8125b-2.fw`, and `rtl8125d-1.fw` from the
  linux-firmware `rtl_nic` directory, plus the license file WHENCE cites for
  them, into
  `image/os/buildroot/board/obake/qdl/rootfs-overlay/lib/firmware/rtl_nic/`.
  Verify the three blobs and the license file exist at that path and no
  other linux-firmware files were added.

## 3. Headless config check

- [x] 3.1 Run the `qdl` kernel `olddefconfig` (the existing image kernel
  configure, not a hardware boot) and verify the built `.config` has
  `CONFIG_R8169=m`, does not set `CONFIG_IGC` or `CONFIG_NET_VENDOR_INTEL`,
  and still sets `CONFIG_X86_INTEL_LPSS=y`, `CONFIG_INTEL_MEI=m`, and
  `CONFIG_HW_RANDOM_INTEL=y`. No shape version bump: this change does not
  touch a shape dependency.
