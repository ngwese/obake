# Spec Delta

## ADDED Requirements

### Requirement: qdl Ethernet is RTL8125

The `qdl` kernel SHALL enable the in-tree Realtek `r8169` driver, which binds
PCI ID `10ec:8125`, and SHALL NOT enable an Intel Ethernet driver.

#### Scenario: r8169 enabled and Intel Ethernet omitted

- **WHEN** a `qdl` image is built
- **THEN** the built kernel configuration enables `r8169` as a module and does
  not enable `igc` or other Intel Ethernet drivers

#### Scenario: Platform Intel drivers remain

- **WHEN** a `qdl` image is built
- **THEN** the built kernel configuration still enables the Intel LPSS, MEI,
  and hardware RNG support the board uses for non-Ethernet devices

### Requirement: qdl RTL8125 PHY firmware

The `qdl` image SHALL include the RTL8125 PHY firmware files that `r8169`
loads when an interface opens, together with their license.

#### Scenario: Firmware blobs are in the image

- **WHEN** a `qdl` image is built
- **THEN** the root filesystem contains `rtl8125a-3.fw`, `rtl8125b-2.fw`, and
  `rtl8125d-1.fw` under the kernel firmware directory, and the license that
  covers those files
