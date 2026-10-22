# Spec Delta

## ADDED Requirements

### Requirement: qdl Ethernet is r8125

The `qdl` image SHALL provide Ethernet for PCI ID `10ec:8125` with Realtek's
out-of-tree `r8125` driver, and SHALL NOT include the in-tree `r8169` driver
or an Intel Ethernet driver. Images for other boards SHALL NOT include
`r8125`.

#### Scenario: r8125 shipped and r8169 omitted

- **WHEN** a `qdl` image is built
- **THEN** the root filesystem contains the `r8125` kernel module, and the
  built kernel configuration does not enable `r8169`, `igc`, or other Intel
  Ethernet drivers

#### Scenario: Other boards omit r8125

- **WHEN** an image is built for a board other than `qdl`
- **THEN** the root filesystem does not contain the `r8125` kernel module

#### Scenario: Platform Intel drivers remain

- **WHEN** a `qdl` image is built
- **THEN** the built kernel configuration still enables the Intel LPSS, MEI,
  and hardware RNG support the board uses for non-Ethernet devices

### Requirement: qdl RTL8125 hardware timestamping

A `qdl` RTL8125 interface driven by `r8125` SHALL expose a PTP hardware clock
and SHALL support two-step transmit hardware timestamps and receive hardware
timestamps for PTPv2 event messages, including L4 sync and delay request.

#### Scenario: Timestamp capabilities report a hardware clock

- **WHEN** timestamping capabilities are queried on a `qdl` RTL8125 interface
- **THEN** the interface reports hardware transmit and receive timestamping
  and a PTP hardware clock index

#### Scenario: Two-step PTPv2 event timestamps can be enabled

- **WHEN** an operator enables two-step transmit hardware timestamps and a
  PTPv2 event receive filter on a `qdl` RTL8125 interface
- **THEN** the request succeeds and the interface accepts those settings

## REMOVED Requirements

### Requirement: qdl Ethernet is RTL8125

**Reason**: The in-tree `r8169` driver does not implement hardware
timestamping. `qdl` Ethernet moves to Realtek's out-of-tree `r8125` driver.
**Migration**: Use the added requirement `qdl Ethernet is r8125`. Platform
Intel LPSS, MEI, and hardware RNG support is unchanged. No persist-partition
migration.

### Requirement: qdl RTL8125 PHY firmware

**Reason**: `r8169` is no longer built for `qdl`. `r8125` carries its PHY
firmware inside the module, so the separate `rtl8125a-3`, `rtl8125b-2`, and
`rtl8125d-1` files are unused.
**Migration**: Remove those firmware files and their `r8169` license from the
`qdl` rootfs overlay. Installed hosts receive firmware with the `r8125`
module in the updated image. No persist-partition migration.
