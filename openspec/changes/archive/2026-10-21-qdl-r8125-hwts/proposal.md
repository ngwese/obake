# Proposal

## Why

AES67/RAVENNA on the MeLE Quieter DL needs a PTP hardware clock on its
RTL8125B ports. The in-tree `r8169` driver in the pinned 7.2.9 kernel has
no hardware-timestamp implementation, so `ethtool -T` cannot report a PHC
and `ptp4l` cannot run with hardware timestamps on `qdl`.

## What Changes

- **BREAKING** for `qdl` images: Ethernet on PCI `10ec:8125` is provided by
  Realtek's out-of-tree `r8125` 9.018.00 driver, built with hardware
  timestamping, instead of in-tree `r8169`.
- The `qdl` kernel build omits `r8169`. Other boards are unchanged.
- A `qdl` interface exposes a PTP hardware clock and two-step hardware
  timestamps for PTPv2 event messages (the AES67/RAVENNA UDP path).
- PHY firmware for this driver is embedded in the module. The separate
  `r8169` firmware files leave the `qdl` rootfs.
- An installed `qdl` host picks this up through the existing image update.
  Predictable interface names stay based on the PCI slot.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `os-image-build`: `qdl` Ethernet is the out-of-tree `r8125` driver with
  hardware timestamping, and the image no longer builds `r8169` or ships
  the `r8169` PHY firmware files.

## Impact

- `qdl` kernel config, the `qdl` rootfs overlay, and the user-guide note
  that names `r8169` as the `qdl` NIC driver.
- New Buildroot package, enabled only on `qdl`, with the vendor source
  committed via Git LFS inside the recipe.
- OTA: a new `qdl` image is required before hardware timestamping works.
  No persist-partition or network-config migration. Other boards' images
  are unchanged.
- `ptp-tools` can use its hardware-timestamp client on `qdl` after the
  image is installed. QEMU cannot exercise this NIC.
