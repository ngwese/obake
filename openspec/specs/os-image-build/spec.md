# os-image-build Specification

## Purpose
Builds bootable obake OS images from a single pinned Buildroot tree so that
every input is deterministic across rebuilds and boards, and emits versioned,
signed artifacts the update system can install.

## Requirements

### Requirement: Deterministic pinned build

The build SHALL derive every input from a pinned Buildroot release and pinned
package versions and source hashes, so that rebuilding with unchanged pins
selects the same base userland, kernel, and runtime regardless of when or where
the build runs, and SHALL emit a reproducible root filesystem archive as its
canonical artifact.

#### Scenario: Same pins select the same inputs

- **WHEN** the image is rebuilt with unchanged pins at a later time or on
  another host
- **THEN** the same package versions, kernel version, and runtime version are
  selected

#### Scenario: Stable inputs across boards

- **WHEN** an image is rebuilt for a new board from the same pins
- **THEN** the shared base userland, kernel, and runtime are unchanged and only
  the board-specific configuration differs

#### Scenario: Reproducible artifact

- **WHEN** the same pinned inputs are built twice
- **THEN** the resulting root filesystem archive is reproducible

### Requirement: Minimal systemd base userland

The image SHALL provide a minimal, systemd-based userland that includes at
least a shell, common administration tools, and the Singularity runtime needed
by the host contract, and SHALL NOT require a distribution package manager at
runtime.

#### Scenario: Required tools present

- **WHEN** an image is built
- **THEN** the userland provides `bash`, `grep`, `git`, `vi`, `sudo`, `ip`,
  `systemctl`, `journalctl`, `dmesg`, and `singularity`

#### Scenario: No package manager dependency

- **WHEN** the running host is used
- **THEN** no runtime package manager is required for the host contract to
  function

### Requirement: Custom realtime kernel

Each image SHALL include a realtime kernel built from a pinned kernel source
with a project realtime configuration fragment, and SHALL version that kernel
together with the image it is built into.

#### Scenario: Realtime tuning present in image

- **WHEN** an image is built
- **THEN** the included kernel is built from the pinned source with the
  project's realtime configuration and the image reports a kernel version tied
  to the image version

### Requirement: Per-board tuning without forking the base

The build SHALL support hardware-specific tuning for distinct target boards
while sharing a single common base, so that adding or changing a board does not
fork the base system.

#### Scenario: Two boards share one base

- **WHEN** images are built for two different target boards
- **THEN** both images derive from the same base Buildroot configuration and
  differ only in the board's defconfig and overlay

### Requirement: Per-slot unified kernel image

Each image SHALL produce a unified kernel image (UKI) bundling the kernel,
initial ramdisk, and kernel command line for the slot the image targets.

#### Scenario: UKI corresponds to its slot

- **WHEN** an image is built for a given slot
- **THEN** the emitted UKI embeds the kernel and command line for that slot and
  is named so the boot entry can associate it with that slot

### Requirement: Versioned signed artifacts

The build SHALL emit versioned image artifacts together with a signature that
the update system can verify before installing them.

#### Scenario: Signed artifact verifiable

- **WHEN** a build completes
- **THEN** it produces a versioned image artifact and a signature over it that
  the update system can validate

### Requirement: Singularity CE runtime built from source

The build SHALL provide a pinned Singularity CE runtime built from source, so
that the runtime does not depend on a distribution package.

#### Scenario: Pinned Singularity CE present

- **WHEN** an image is built
- **THEN** the runtime provides the pinned Singularity CE version built from
  source rather than a distribution package

### Requirement: Kernel namespaces for container runtime

Every board kernel SHALL enable Linux namespaces including user, PID, UTS,
IPC, network, and cgroup namespaces, so the non-setuid Singularity runtime can
create the namespaces it needs to mount and enter containers.

#### Scenario: User namespace supported

- **WHEN** a built image boots
- **THEN** creating a user namespace succeeds for an unprivileged process
  (for example the `obake` login user)

#### Scenario: Companion namespaces enabled

- **WHEN** a board kernel is built with the shared host fragment
- **THEN** PID, UTS, IPC, network, and cgroup namespaces are available in
  addition to user namespaces

### Requirement: Kernel cgroup controllers for container runtime

Every board kernel SHALL enable cgroup support with the common resource
controllers needed by container workloads (memory, CPU accounting, cpusets,
devices, and PIDs), without enabling realtime group scheduling that conflicts
with the project's PREEMPT_RT configuration.

#### Scenario: Core cgroup controllers present

- **WHEN** a built image boots
- **THEN** cgroup controllers for memory, CPU accounting, cpusets, devices,
  and PIDs are available

#### Scenario: Realtime group scheduling omitted

- **WHEN** a board kernel is built with the shared host and realtime fragments
- **THEN** realtime group scheduling is not enabled

### Requirement: Singularity runnable as host login user

The image SHALL allow the `obake` login user to run a Singularity container
from a local SIF without requiring a setuid helper or a root login, using the
kernel namespace and FUSE support already required by the non-setuid runtime.

#### Scenario: Local SIF runs as obake

- **WHEN** the `obake` user runs `singularity run` (or equivalent) against a
  local SIF on a booted image
- **THEN** the container starts successfully without a user-namespace error

### Requirement: Local display console for hardware boards

A board with no serial console SHALL include a kernel build that drives a text
console from the firmware-provided EFI framebuffer, so an attached monitor and
USB keyboard can be used without a GPU driver that the realtime configuration
excludes.

#### Scenario: Firmware framebuffer text console

- **WHEN** the qdl image is built
- **THEN** the kernel binds a framebuffer console to the EFI framebuffer and
  provides a virtual terminal for local output

#### Scenario: Console does not require an RT-incompatible GPU driver

- **WHEN** the qdl kernel is built with the realtime fragment
- **THEN** the local console works without i915, which the realtime
  configuration excludes

### Requirement: Board build profiles

Each board SHALL provide a `default` and a `virtual` build profile. A profile
supplies the board's installer and console build defaults (`MAKE_INSTALLER`,
`CONSOLE`, `INSTALLER_TARGET_DISK`); `default` targets the physical board and
`virtual` targets an emulation or verification target, and the generic
boards' two profiles are identical. When no profile is named the build SHALL
use `default`, an explicit setting SHALL override the profile value, and an
unknown profile SHALL fail the build.

#### Scenario: Default profile selected

- **WHEN** a board is built without naming a profile
- **THEN** it applies the `default` profile's installer and console defaults

#### Scenario: Virtual profile selected

- **WHEN** a board is built naming the `virtual` profile
- **THEN** it applies that profile's emulation-oriented installer and console
  defaults

#### Scenario: Explicit setting overrides the profile

- **WHEN** an installer or console setting is also provided explicitly with a
  profile
- **THEN** the explicit value takes precedence over the profile value

#### Scenario: Generic profiles are equivalent

- **WHEN** a generic board is built with either profile
- **THEN** the resulting build configuration is the same

#### Scenario: Unknown profile rejected

- **WHEN** a build names a profile that the board does not provide
- **THEN** the build fails with an error naming the supported profiles

### Requirement: Profile console for installer and installed OS

The build-time `CONSOLE` setting (`serial` or `local`) SHALL select the
interactive console for both the installer medium and the installed OS
unified kernel images, including the kernel command line and getty, so a
board profile's console choice is consistent from install through first boot.

#### Scenario: Local console on installer and installed OS

- **WHEN** a board is built with `CONSOLE=local`
- **THEN** the installer medium and each installed OS UKI present on the local
  text console

#### Scenario: Serial console on installer and installed OS

- **WHEN** a board is built with `CONSOLE=serial`
- **THEN** the installer medium and each installed OS UKI present on the
  board's serial console

#### Scenario: qdl default uses local end-to-end

- **WHEN** the qdl board is built with its default profile
- **THEN** both the installer medium and the installed OS UKIs use the local
  text console

### Requirement: Board-implied target architecture

Building a board SHALL derive the target architecture from the board's
configuration, so the operator does not supply an architecture separately.

#### Scenario: Architecture follows the board

- **WHEN** a board is built without an architecture argument
- **THEN** the emitted image targets the architecture declared by that board

#### Scenario: Distinct architectures from distinct boards

- **WHEN** a board that targets x86_64 and a board that targets aarch64 are
  built the same way
- **THEN** each emitted image targets its own board's architecture

### Requirement: Optional build-time obake password

The build SHALL accept an optional cleartext password for the `obake` login
account via the `OBKA_PASSWORD` environment variable. When unset or empty, the
account SHALL have no password and SSH password authentication as `obake`
SHALL fail. When set, the built image SHALL accept that password for local
console login and SHALL allow SSH password authentication as `obake` with that
password. The password SHALL live only in the image rootfs, not on the persist
partition.

#### Scenario: Default has no password

- **WHEN** an image is built without `OBKA_PASSWORD` set
- **THEN** the `obake` account has no usable password and SSH password
  authentication as `obake` fails

#### Scenario: Password set enables console and SSH password login

- **WHEN** an image is built with a non-empty `OBKA_PASSWORD`
- **THEN** the `obake` account accepts that password for local console login
  and an SSH client can authenticate as `obake` using that password

#### Scenario: Password is not persist state

- **WHEN** an image built with `OBKA_PASSWORD` is installed
- **THEN** the password hash is not written to the persist partition and is
  replaced when a different slot image is activated

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
hardware that enables USB MIDI 2.0 (UMP), NVMe, and AOIP network plumbing
(PTP clock, PHY timestamping, XDP sockets), SHALL use the shared realtime
fragment including idle tickless operation, and SHALL NOT enable wireless
drivers. Ethernet NIC selection for this board is owned by a later change
that matches the enumerated hardware.

#### Scenario: MIDI 2.0 present

- **WHEN** a `qdl` image is built
- **THEN** the built kernel configuration enables USB MIDI 2.0 support

#### Scenario: Realtime idle tickless retained

- **WHEN** a `qdl` image is built
- **THEN** the built kernel configuration enables PREEMPT_RT and NO_HZ_IDLE
  from the shared realtime fragment

#### Scenario: Wireless omitted

- **WHEN** a `qdl` image is built
- **THEN** the built kernel configuration does not enable Wi-Fi or Bluetooth
  drivers

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

### Requirement: System CA trust store

The image SHALL include a system CA trust store that TLS clients on the host
(including Singularity and curl) use by default to verify server certificates
for public HTTPS endpoints, without requiring a package manager at runtime.

#### Scenario: Public HTTPS verifies with the system store

- **WHEN** an image is built and a host TLS client connects to a server whose
  certificate chains to a well-known public CA
- **THEN** certificate verification succeeds using the image's system trust
  store

#### Scenario: Trust store present without a package manager

- **WHEN** an image is built
- **THEN** the root filesystem contains the system CA trust store and no
  runtime package manager is required to obtain it

### Requirement: Build-time extra root CAs

The build SHALL accept zero or more additional root CA certificates supplied
at image build time and SHALL merge them into the system CA trust store so
TLS clients trust servers signed by those roots (for example a personal
container registry).

#### Scenario: Extra root CA trusts a private endpoint

- **WHEN** an image is built with an additional root CA certificate supplied
  to the build
- **THEN** a host TLS client verifies a server certificate signed by that
  root using the system trust store

#### Scenario: No extras leaves the default store

- **WHEN** an image is built without additional root CA certificates
- **THEN** the system trust store contains only the default public CA set
  and the build still succeeds

### Requirement: Removable filesystem support including FAT family and exFAT

The shared base image SHALL include kernel and userspace support needed to
mount common removable filesystems used for update media, including MS-DOS
FAT (`msdos`), the generic FAT stack, FAT32/VFAT (`vfat`), and exFAT, so
attaching such media does not fail with an unknown filesystem type.

#### Scenario: FAT family kernel options enabled

- **WHEN** an image is built
- **THEN** the kernel configuration enables `CONFIG_FAT_FS`,
  `CONFIG_MSDOS_FS`, and `CONFIG_VFAT_FS` for the ESP and for
  MS-DOS/FAT/FAT32 removable media

#### Scenario: exFAT mount type is known

- **WHEN** an image is built and a removable exFAT volume is attached
- **THEN** the system recognizes filesystem type `exfat` and can mount the
  volume (via the removable-media auto-mount path)

### Requirement: obake-os release identity

The built image SHALL present as `obake-os` in `/etc/os-release`: `NAME` and
`ID` SHALL be `obake-os`; `VERSION` and `VERSION_ID` SHALL equal the contents
of `image/os/buildroot/VERSION` (or `OBAKE_IMAGE_VERSION` when set); and
`PRETTY_NAME` SHALL be `obake-os` followed by that version string.

#### Scenario: os-release fields

- **WHEN** an image is built
- **THEN** `/etc/os-release` has `NAME=obake-os`, `ID=obake-os`,
  `VERSION` and `VERSION_ID` matching the image version, and
  `PRETTY_NAME="obake-os <version>"`

#### Scenario: Aligns with image-version

- **WHEN** an image is built
- **THEN** os-release `VERSION` equals the contents of
  `/etc/obake/image-version`

### Requirement: obake-os login banner

The built image SHALL display `Welcome to obake-os` as the login banner
(`/etc/issue`) instead of Buildroot's default welcome string.

#### Scenario: Login issue text

- **WHEN** an image is built
- **THEN** `/etc/issue` contains `Welcome to obake-os` and does not contain
  `Welcome to Buildroot`

### Requirement: Kernel release board and revision suffixes

Each image's kernel release string (`uname -r`) SHALL end with
`-g<shortsha>` or `-g<shortsha>-dirty`, where `<shortsha>` is the short git
commit of the image tree used for the build and `-dirty` is present when that
tree has uncommitted changes. Boards that define a LOCALVERSION suffix SHALL
include that suffix before the `-g…` segment: `-qdl` for `qdl`, `-generic`
for `generic-x86_64` and `generic-aarch64`. The `rk35xx` board SHALL omit a
board LOCALVERSION suffix for now.

#### Scenario: qdl kernel release shape

- **WHEN** a `qdl` image is built from a clean image git tree at commit
  `<shortsha>`
- **THEN** `uname -r` matches
  `<pinned-kernel-version>-qdl-g<shortsha>`

#### Scenario: generic board kernel release shape

- **WHEN** a `generic-x86_64` or `generic-aarch64` image is built from a clean
  image git tree at commit `<shortsha>`
- **THEN** `uname -r` matches
  `<pinned-kernel-version>-generic-g<shortsha>`

#### Scenario: rk35xx has no board suffix yet

- **WHEN** an `rk35xx` image is built from a clean image git tree at commit
  `<shortsha>`
- **THEN** `uname -r` matches
  `<pinned-kernel-version>-g<shortsha>` and does not contain `-rk35xx`

#### Scenario: Dirty tree marks the kernel

- **WHEN** an image is built from an image git tree with uncommitted changes
- **THEN** `uname -r` ends with `-dirty`
