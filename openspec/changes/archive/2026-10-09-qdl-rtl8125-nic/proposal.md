# Proposal

## Why

The `qdl` kernel enables the Intel I226 driver (`igc`), but the MeLE Quieter DL
on the bench enumerates two Realtek RTL8125 controllers (`10ec:8125`) and no
`8086:125c` device. With no driver bound, the host boots to a local login and
never creates an Ethernet interface, so DHCP cannot start.

## What Changes

- Stop enabling Intel Ethernet in the `qdl` kernel (`CONFIG_IGC` and
  `CONFIG_NET_VENDOR_INTEL`). Leave other Intel platform drivers (LPSS, MEI,
  hardware RNG) in place.
- Enable the in-tree Realtek family driver `r8169`, which claims `10ec:8125`,
  as a module.
- Ship the RTL8125 PHY firmware blobs `r8169` loads when the interface opens
  (`rtl8125a-3`, `rtl8125b-2`, `rtl8125d-1`), with their license.
- Correct docs that describe the `qdl` NIC as an Intel I226.
- This supersedes the I226/IGC enablement in the still-open `add-qdl-board`
  delta. That wording is not in the main `os-image-build` spec.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `os-image-build`: the `qdl` kernel provides RTL8125 Ethernet via `r8169` and
  its PHY firmware, and does not enable an Intel Ethernet driver.

## Impact

- `image/os/buildroot/board/obake/qdl/linux.config` and the `qdl` rootfs
  overlay (firmware blobs). Docs that name the I226 as the `qdl` NIC.
- No change to partition layout, persist contents, OTA mechanism, or shape
  portability. An already-installed `qdl` host needs a new image before its
  RTL8125 ports appear as Linux interfaces.
- Other boards are unchanged. QEMU verification still cannot exercise this
  NIC; link-up stays a hardware check.
