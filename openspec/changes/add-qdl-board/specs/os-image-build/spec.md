# Spec Delta

## ADDED Requirements

### Requirement: qdl board target

The build SHALL support a `qdl` board for the MeLE Quieter DL that shares the
common Buildroot base and pinned kernel version with other boards, and SHALL
record board identity `qdl` and architecture `x86_64` in the image.

#### Scenario: qdl image shares the pinned base

- **WHEN** an image is built for board `qdl`
- **THEN** it uses the same pinned Buildroot release, kernel version, and
  shared base configuration as other boards, differing only in the `qdl`
  board fragment, overlay, and kernel config

#### Scenario: qdl board identity baked into the image

- **WHEN** an image is built for board `qdl`
- **THEN** the root filesystem reports `board=qdl` and `arch=x86_64` under
  `/etc/obake/board`

### Requirement: qdl kernel hardware and MIDI 2.0 enablement

The `qdl` board kernel SHALL be a minimal custom config for Quieter DL
hardware that enables USB MIDI 2.0 (UMP), Intel I226 ethernet (IGC), NVMe,
and AOIP network plumbing (PTP clock, PHY timestamping, XDP sockets), SHALL
use the shared realtime fragment including idle tickless operation, and SHALL
NOT enable wireless drivers.

#### Scenario: MIDI 2.0 and IGC present

- **WHEN** a `qdl` image is built
- **THEN** the built kernel configuration enables USB MIDI 2.0 support and
  the Intel IGC driver as a module

#### Scenario: Realtime idle tickless retained

- **WHEN** a `qdl` image is built
- **THEN** the built kernel configuration enables PREEMPT_RT and NO_HZ_IDLE
  from the shared realtime fragment

#### Scenario: Wireless omitted

- **WHEN** a `qdl` image is built
- **THEN** the built kernel configuration does not enable Wi-Fi or Bluetooth
  drivers
