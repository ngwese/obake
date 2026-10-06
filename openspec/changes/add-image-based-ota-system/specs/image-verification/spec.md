# Spec Delta

## Purpose

Gives the project a reproducible way to build obake images on its development
hosts and a headless way to prove that an image installs, boots, and functions
before it is released, so regressions are caught without physical hardware.

## ADDED Requirements

### Requirement: Reproducible containerized build environment

The build SHALL run inside a containerized Linux environment and SHALL produce
x86_64 images from both a macOS arm64 host and a Windows 11 WSL x86_64 host
without host-specific manual setup.

#### Scenario: Build on macOS arm64

- **WHEN** the build runs on a macOS arm64 host
- **THEN** it produces an x86_64 image through the containerized environment
  with no manual host configuration

#### Scenario: Build on Windows 11 WSL x86_64

- **WHEN** the build runs on a Windows 11 WSL x86_64 host
- **THEN** it produces an x86_64 image through the same containerized
  environment

#### Scenario: Equivalent output across hosts

- **WHEN** the same pinned inputs are built on both host types
- **THEN** the resulting x86_64 image artifacts are equivalent

### Requirement: Primary architecture

The verification SHALL target x86_64 as the primary architecture, while images
for other supported architectures are built from the same layers.

#### Scenario: Verification targets x86_64

- **WHEN** the verification harness runs
- **THEN** it builds and tests an x86_64 image by default

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
