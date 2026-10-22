# Design

## Context

See proposal.md for why. `board/obake/qdl/linux.config` sets
`CONFIG_R8169=m`, `CONFIG_PTP_1588_CLOCK=y`, and
`CONFIG_NETWORK_PHY_TIMESTAMPING=y`. The pinned kernel is 7.2.9 with the
shared `PREEMPT_RT` fragment. In that tree, `r8169` has no PHC and no
hardware-timestamp ops. Realtek `r8125` 9.018.00 implements PTP behind
`ENABLE_PTP_SUPPORT`, which defaults to off. Its newest `LINUX_VERSION`
guard is 6.13. On 7.2, `SIOCSHWTSTAMP` is delivered only through
`ndo_hwtstamp_set`; the driver's `ndo_eth_ioctl` path is not called.
RTL8125B (`CFG_METHOD_4` and `CFG_METHOD_5`) sets `HwSuppPtpVer = 3` when
PTP is compiled in. The `qdl` overlay currently ships `rtl8125a-3.fw`,
`rtl8125b-2.fw`, `rtl8125d-1.fw`, and `LICENSE.r8169` for `r8169`.

## Goals / Non-Goals

**Goals:**

- Build `r8125` only into the `qdl` image, with PTP and the AoIP module
  flags below.
- Leave `r8169` out of the `qdl` kernel so `10ec:8125` has one driver.
- Bridge kernel 7.2's hardware-timestamp netdev ops onto the vendor ioctl
  helpers.
- Store the vendor tarball as a Git LFS object inside the recipe.

**Non-Goals:**

- Patching PTP into in-tree `r8169`.
- NIC changes on other boards.
- One-step timestamping, VLAN-tagged PTP, or raw 802.3 PTP. The 8125B
  parser does not implement those.
- RSS or multiple TX queues.
- A PREEMPT_RT rewrite of the vendor driver.
- A QEMU test of this PHY. Hardware timestamp checks stay on the board.
- Changes to the `ptp-tools` shape.

## Decisions

1. **Buildroot kernel-module package `package/r8125`**, selected with
   `BR2_PACKAGE_R8125=y` only in `configs/obake-qdl.config`.
   `external.mk` already includes `package/*/*.mk`. Build in `src/`
   (`R8125_MODULE_SUBDIRS = src`). Command-line make variables override the
   Makefile's `ENABLE_* = n` lines, so pass flags as
   `R8125_MODULE_MAKE_OPTS`. Alternative: a blacklist of `r8169` while both
   modules ship (udev can still bind `r8169` first).

2. **Vendor source lives in the recipe and is a Git LFS object.** Copy
   `r8125-9.018.00.tar.bz2` to
   `image/os/buildroot/package/r8125/` and track it from `image/.gitattributes`
   (`os/buildroot/package/r8125/*.tar.bz2`). The package `SITE` is that
   directory. The file is about 122 KiB; LFS is still how this pin is
   stored. Alternative: drop it in `vendor/dl` (also LFS, but not in the
   recipe).

3. **Module flags for this board:**

   | Flag | Value | Why |
   |---|---|---|
   | `ENABLE_PTP_SUPPORT` | y | PHC and hardware timestamps |
   | `ENABLE_EEE` | n | LPI adds latency jitter |
   | `CONFIG_ASPM` | n | PCIe L1 adds latency |
   | `CONFIG_SOC_LAN` | n | That flag skips PCIe MRRS setup; this NIC is a PCIe endpoint |
   | `ENABLE_DASH_SUPPORT` | n | Out-of-band management, unused here |
   | `ENABLE_MULTIPLE_TX_QUEUE` | n | One TX ring keeps the single TX timestamp engine |
   | `ENABLE_RSS_SUPPORT` | n | Same, and this board does not need RSS for PTP |
   | `ENABLE_USE_FIRMWARE_FILE` | n | PHY firmware is compiled into the module |

   Leave `ENABLE_TX_NO_CLOSE` and `ENABLE_PAGE_REUSE` at the vendor
   default (`y`).

4. **`# CONFIG_R8169 is not set` in `qdl/linux.config`.** Keep
   `CONFIG_NET_VENDOR_REALTEK=y` and `CONFIG_PTP_1588_CLOCK=y`. `R8169` has
   no `default y`, and an explicit unset matches the existing Intel
   Ethernet disable so `olddefconfig` cannot turn it back on. No modprobe
   blacklist. Alternative: delete the `CONFIG_R8169=m` line only (same
   result today, easier to regress).

5. **One Buildroot patch on the vendor sources** adds `ndo_hwtstamp_get`
   and `ndo_hwtstamp_set`. They call the existing `rtl8125_get_tstamp` /
   `rtl8125_set_tstamp` with the `ifreq` the kernel still hangs off
   `kernel_hwtstamp_config`, and they mark the config copied to user space
   so `dev_set_hwtstamp` does not copy it again. The PTP engine stays in
   the tarball. Alternative: wait for a newer Realtek drop that implements
   the 7.2 ops (none is in hand).

6. **Delete the `qdl` overlay firmware** under
   `rootfs-overlay/usr/lib/firmware/rtl_nic/` (`rtl8125a-3.fw`,
   `rtl8125b-2.fw`, `rtl8125d-1.fw`, `LICENSE.r8169`). With
   `ENABLE_USE_FIRMWARE_FILE` off, the module does not `request_firmware`
   those files. Update `image/docs/user-guide.md`, which still names
   `r8169` as the `qdl` NIC, and the stale `I226-V` comment in
   `obake-qdl.config`.

## Risks / Trade-offs

- **[Risk] 9.018.00 does not compile against 7.2.9** → The first
  implementation step builds the module on the `qdl` kernel. Fix only the
  breaks that build hits, in the same patch series as the timestamp ops.
  The dead `from_timer` / `del_timer_sync` / `hrtimer_init` calls found in
  the sources are inside comments or older `#if` branches.
- **[Risk] `ndo_hwtstamp_*` runs under the netdev ops lock and deadlocks
  if the vendor helper takes `rtnl`** → The current helpers do not take
  `rtnl`. Do not add that lock in the shim. If lockdep reports a cycle,
  fix it in that patch.
- **[Risk] PREEMPT_RT plus a vendor driver** → Build against the RT
  kernel. A load, traffic, and PTP lock check on the Quieter DL is the
  hardware follow-up. This change does not claim the driver is RT-clean.
- **[Trade-off] EEE and ASPM off** → Higher idle power, lower latency
  jitter. Right default for an AoIP appliance.
- **[Trade-off] Embedded firmware replaces the linux-firmware blobs** →
  The blob matches driver 9.018.00. Rolling the driver forward means
  rolling this tarball.

## Migration Plan

- Rebuild and reinstall `qdl`, or ship the image through the existing
  update path. Interface names stay PCI-slot names. No persist migration.
- Rollback: restore `CONFIG_R8169=m`, drop `BR2_PACKAGE_R8125`, and restore
  the overlay firmware files.

## Open Questions

None.
