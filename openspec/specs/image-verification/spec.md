# image-verification Specification

## Purpose
Gives the project a reproducible way to build obake images on its development
hosts and a headless way to prove that an image installs, boots, and functions
before it is released, so regressions are caught without physical hardware.

## Requirements

### Requirement: Build without target emulation

The build SHALL produce target-architecture images without executing
target-architecture binaries and without relying on target-architecture
emulation.

#### Scenario: Any Linux host can build any target

- **WHEN** the build runs on a Linux host whose architecture differs from the
  target
- **THEN** it produces the target image without invoking a target-architecture
  emulator or binfmt handler

### Requirement: Decoupled build and verification

The build SHALL emit a self-contained artifact set sufficient to install and
boot an image, together with a manifest of the versions and hashes of those
artifacts, and verification SHALL consume that artifact set independently of the
build host.

#### Scenario: Build emits installable artifacts

- **WHEN** the build completes
- **THEN** it emits the bootable image artifacts and a manifest of their
  versions and hashes

#### Scenario: Verify on a host other than the build host

- **WHEN** verification runs on a host other than the build host
- **THEN** it consumes the emitted artifact set without rebuilding the image

#### Scenario: Best available accelerator

- **WHEN** verification runs on a host
- **THEN** it uses the fastest accelerator that host provides and falls back to
  software emulation when none is available

### Requirement: Reproducible containerized build environment

The build SHALL run inside a containerized Linux environment native to the host
architecture and SHALL produce target images by cross-compilation without
host-specific manual setup, including from a macOS arm64 host through a native
arm64 container.

#### Scenario: Build on macOS arm64

- **WHEN** the build runs on a macOS arm64 host
- **THEN** it produces a target image through a native arm64 container without
  target-architecture emulation

#### Scenario: Build on Windows 11 WSL x86_64

- **WHEN** the build runs on a Windows 11 WSL x86_64 host
- **THEN** it produces a target image through the same containerized
  environment

#### Scenario: Equivalent output across hosts

- **WHEN** the same pinned inputs are built on two host types
- **THEN** the resulting target image artifacts are equivalent

### Requirement: Primary architecture

The verification SHALL build images for both x86_64 and aarch64 targets from the
same build, and on a host that cannot natively execute a target's binaries SHALL
use the host-native architecture as a hardware proxy.

#### Scenario: Verification targets x86_64

- **WHEN** the verification harness runs on an x86_64 host
- **THEN** it builds and tests an x86_64 image

#### Scenario: Native-first verification

- **WHEN** the verification harness runs on an arm64 host that cannot natively
  execute the x86_64 image
- **THEN** it builds and verifies the aarch64 image as a proxy for arm64
  hardware

#### Scenario: Both architectures built from one source

- **WHEN** the build runs
- **THEN** x86_64 and aarch64 images are produced from the same build inputs

### Requirement: QEMU boot verification

The verification harness SHALL boot a built image under QEMU using UEFI
firmware and SHALL confirm the system reaches a running state.

#### Scenario: Built image boots under UEFI

- **WHEN** a built image is booted under QEMU with UEFI firmware
- **THEN** the system boots the active slot and reaches a running state

### Requirement: QEMU installer verification

The verification harness SHALL run the installer against a virtual disk under
QEMU, then boot the installed host and confirm it functions.

#### Scenario: Install then boot in QEMU

- **WHEN** the installer runs against an empty virtual disk in QEMU and
  completes
- **THEN** the virtual disk boots the installed host and the host functions as
  expected

### Requirement: Headless deterministic verification

The verification harness SHALL run without a graphical display, drive the serial
or text console, and report a deterministic pass or fail result.

#### Scenario: Headless run reports pass or fail

- **WHEN** the verification harness runs on a host with no display
- **THEN** it completes without a graphical interface and reports a clear pass
  or fail result

### Requirement: Functional checks after boot

After a successful boot, the verification SHALL check core host behavior
including the read-only root and the writable contract, and SHALL exercise an
update apply and revert.

#### Scenario: Core host behavior checked

- **WHEN** the installed host has booted in QEMU
- **THEN** the harness confirms the root is read-only, the writable contract
  behaves as specified, and an update applies and reverts as specified

#### Scenario: Host user access checked

- **WHEN** the installed host has booted in QEMU
- **THEN** the harness confirms the host user can manage units and read the
  kernel log and full system journal
