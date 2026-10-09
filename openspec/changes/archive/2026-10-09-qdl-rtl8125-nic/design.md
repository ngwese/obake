# Design

## Context

See proposal.md for why. The `qdl` kernel config in
`image/os/buildroot/board/obake/qdl/linux.config` sets `CONFIG_IGC=m` and
`CONFIG_NET_VENDOR_INTEL=y`. Curieux's Quieter DL defconfig enables both
`CONFIG_IGC=m` (inside a broad Intel Ethernet block) and `CONFIG_R8169=m`,
and its setup notes name the RTL8125. This machine's Ethernet class devices
are `10ec:8125`. `CONFIG_NET_VENDOR_INTEL` defaults to y, so deleting the
lines without an explicit disable turns the Intel menu back on at
`olddefconfig`. `IGC` itself defaults to n.

## Goals / Non-Goals

**Goals:**

- Bind `r8169` to `10ec:8125` and include the PHY firmware it requests at open.
- Keep the Intel Ethernet family out of the built `qdl` config.
- Keep LPSS, MEI, and the Intel hardware RNG.

**Non-Goals:**

- RTL8125 hardware timestamping (still a Curieux todo; PTP and XDP symbols
  stay as they are).
- The out-of-tree `r8125` DKMS driver.
- The full `linux-firmware` package or any wireless firmware.
- NIC changes on other boards.
- A QEMU test of this PHY.

## Decisions

1. **`CONFIG_R8169=m` plus `CONFIG_NET_VENDOR_REALTEK=y`** — `r8169` is the
   in-tree driver whose PCI table includes `10ec:8125`. Its `select`s pull
   firmware loading, CRC32, and the Realtek PHY. Alternative: vendor `r8125`
   (extra out-of-tree module, not what the pinned kernel already contains).

2. **Explicit `# CONFIG_NET_VENDOR_INTEL is not set`** — removes `igc` and
   the rest of the Intel Ethernet menu. Alternative: delete only `CONFIG_IGC`
   (the menu defaults back to y; no Intel driver would be selected, but the
   family would not be off).

3. **Three firmware files in the `qdl` rootfs overlay** — copy
   `rtl8125a-3.fw`, `rtl8125b-2.fw`, and `rtl8125d-1.fw` plus the license
   WHENCE names for them into
   `board/obake/qdl/rootfs-overlay/usr/lib/firmware/rtl_nic/` (merged `/usr`;
   a real `lib/` directory fails Buildroot's overlay check). `r8169` picks the
   file from the chip revision at open. Alternative: `BR2_PACKAGE_LINUX_FIRMWARE_RTL_8169`
   (downloads the whole linux-firmware tarball, which is not in `vendor/dl`).

4. **Amend the unarchived `add-qdl-board` NIC wording** — that change's delta
   still requires IGC, and it is not in the main spec. Update that scenario
   so a later archive does not add an IGC requirement next to this one.
   Leave its MIDI 2.0 text in place.

## Risks / Trade-offs

- **[Risk] A later Quieter DL SKU really has an I226** → Mitigation: this
  change follows the board that was enumerated (`10ec:8125`) and Curieux's
  RTL8125 notes. An I226 unit would need `igc` added back.
- **[Risk] Wrong license file shipped with the blobs** → Mitigation: copy
  the license WHENCE cites for those three files, not a guessed name.
- **[Risk] PHY still links at 100 Mbps after the driver binds** → Mitigation:
  firmware is what programs 2.5G advertisement. Cable and switch checks stay
  a hardware follow-up, outside this change.
- **[Trade-off] Firmware lives in git rather than the download cache** → The
  blobs are small and specific; the linux-firmware tarball is not.

## Migration Plan

- Rebuild and reinstall `qdl` (or ship the new image through the existing
  update path). No persist or partition migration.
- Rollback: restore `CONFIG_IGC` and drop the firmware files.

## Open Questions

None.
