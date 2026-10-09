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

### Requirement: Build tooling separate from verification

The build entry point and its containerized build environment SHALL be provided
outside the verification tree, so that build and verification are decoupled and
verification consumes the emitted artifact set rather than build internals.

#### Scenario: Build entry point outside verification

- **WHEN** an operator looks for the build tooling
- **THEN** the build entry point and build environment live in a dedicated
  build area separate from the verification harnesses

#### Scenario: Verification consumes artifacts only

- **WHEN** verification runs
- **THEN** it consumes the artifact directory and does not depend on the build
  tooling's internal layout

### Requirement: Build outputs within the image repository

Every host-visible build output directory SHALL live inside the image
repository, SHALL have a name beginning with an underscore, and SHALL be
ignored by version control.

#### Scenario: Outputs inside the repository

- **WHEN** the build runs from a clone of the image repository alone
- **THEN** every output directory is created inside that repository tree

#### Scenario: Underscore directories ignored

- **WHEN** output directories have been created by a build
- **THEN** version control ignores them

### Requirement: Host-owned build outputs

Files written from the build container to the host SHALL be owned by the user
who invoked the build, so the user can read, move, and remove them without
elevated privileges.

#### Scenario: Container outputs owned by the invoking user

- **WHEN** a containerized build completes
- **THEN** the staged files on the host are owned by the invoking user

#### Scenario: Cleanup needs no elevated privileges

- **WHEN** the invoking user removes a completed build's output directory
- **THEN** the removal succeeds without elevated privileges

### Requirement: Unique per-board artifact directory

Each build SHALL stage its artifacts in a directory whose path identifies the
board, the profile, and the build revision, so that builds for different boards
or profiles do not overwrite one another.

#### Scenario: Two boards of one architecture do not collide

- **WHEN** two boards that share a target architecture are built
- **THEN** their artifact directories are distinct

#### Scenario: Virtual and default do not collide

- **WHEN** the same board is built with the `virtual` and `default` profiles
- **THEN** their artifact directories are distinct

#### Scenario: Revision recorded in the path

- **WHEN** the image repository has no uncommitted changes
- **THEN** the artifact directory path includes the image version and the short
  revision; when it has uncommitted changes the short revision is marked dirty

### Requirement: Build reports artifact locations

The build SHALL print the location of the artifacts it produces when it
completes, and SHALL be able to report the artifact directory without
performing a build.

#### Scenario: Locations printed

- **WHEN** a build completes
- **THEN** it prints the artifact directory and the produced artifact paths

#### Scenario: Directory reported without building

- **WHEN** the artifact directory is requested without building
- **THEN** the build prints the path and does not run a build
