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
with a project realtime configuration fragment, and SHALL record that kernel's
release in the image. The recorded release identifies the kernel configuration
revision and SHALL NOT encode the image git revision.

#### Scenario: Realtime tuning present in image

- **WHEN** an image is built
- **THEN** the included kernel is built from the pinned source with the
  project's realtime configuration, and the image's recorded kernel release
  matches `uname -r` and does not encode the image git revision

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
IPC, network, and cgroup namespaces, so Singularity can create namespaces for
fakeroot builds, optional unprivileged flows, and container entry.

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

The image SHALL ship a setuid SingularityCE helper and SHALL allow the `obake`
login user to run a local SIF using kernel SquashFS mounts (not a FUSE
squashfuse mount as the primary path), without requiring a root login.

#### Scenario: Local SIF runs as obake

- **WHEN** the `obake` user runs `singularity run` (or equivalent) against a
  local SIF on a booted image
- **THEN** the container starts successfully and the SIF root filesystem is
  mounted with the kernel SquashFS driver rather than squashfuse/FUSE

#### Scenario: Setuid helper present

- **WHEN** a built image is inspected
- **THEN** the Singularity setuid starter binary is present, owned by root, and
  mode `4755`, and `singularity.conf` is owned by root and not writable by
  non-root users, with setuid and kernel SquashFS mounts allowed

### Requirement: Fakeroot maps for on-device Singularity builds

The image SHALL provision subordinate UID and GID maps for the `obake` login
user so that user can run Singularity with `--fakeroot` on the device (for
example `singularity build --fakeroot` or `singularity exec --fakeroot`).

#### Scenario: subuid and subgid present for obake

- **WHEN** a built image boots
- **THEN** `/etc/subuid` and `/etc/subgid` each contain a mapping for `obake`
  with at least 65536 IDs

#### Scenario: Fakeroot exec as obake

- **WHEN** the `obake` user runs `singularity exec --fakeroot` against a local
  SIF with a command that prints the effective user id
- **THEN** the command reports uid `0` inside the container

### Requirement: Filesystem extended attribute support

Every board kernel SHALL enable extended-attribute support for each onboard
filesystem that exposes a kernel option for it among the filesystems the image
already requires: SquashFS xattrs, ext4 POSIX ACLs and security xattrs, and
tmpfs xattrs (with tmpfs POSIX ACLs). Filesystems without a Linux xattr Kconfig
knob (for example FAT/exFAT) are unchanged. Overlay and FUSE SHALL continue to
rely on their stacked or userspace xattr paths without needing a separate
host-only xattr toggle.

#### Scenario: SquashFS xattrs enabled

- **WHEN** a board kernel is built with the shared host fragment
- **THEN** SquashFS extended-attribute support is enabled

#### Scenario: ext4 ACL and security xattrs enabled

- **WHEN** a board kernel is built with the shared host fragment
- **THEN** ext4 POSIX ACL support and ext4 security-label (security xattr)
  support are enabled

#### Scenario: tmpfs xattrs enabled

- **WHEN** a board kernel is built with the shared host fragment
- **THEN** tmpfs extended-attribute support is enabled

#### Scenario: SIF with xattrs mounts without ignore errors

- **WHEN** a booted image mounts a SquashFS-backed SIF that contains extended
  attributes using the kernel SquashFS driver
- **THEN** the kernel does not log that SquashFS xattrs will be ignored or that
  the xattr id index table cannot be read

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

### Requirement: Installer product emits installer media

When the build is invoked with the installer product, the build SHALL force
installer media generation regardless of the board profile's default
`MAKE_INSTALLER` value, while still applying the profile's console and
target-disk defaults unless explicitly overridden. It SHALL NOT emit signed
RAUC update bundles unless bundle generation is explicitly requested. Staged
installer media SHALL be only the installer image this build produced.

#### Scenario: Profile installer off still emits media with installer product

- **WHEN** a board whose profile defaults `MAKE_INSTALLER` to off is built
  with the installer product
- **THEN** the staged artifacts include the installer medium

#### Scenario: Profile console still applies

- **WHEN** a board is built with the installer product and no console override
- **THEN** the installer and installed OS use the profile's console selection

#### Scenario: Installer product omits update bundles

- **WHEN** a board is built with the installer product and bundle generation
  is not explicitly requested
- **THEN** the staged artifacts do not include `.raucb` bundles

#### Scenario: Explicit bundle request still emits bundles

- **WHEN** a board is built with the installer product and bundle generation
  is explicitly requested
- **THEN** the staged artifacts include signed `.raucb` bundles

#### Scenario: Stale installer images are not staged

- **WHEN** the Buildroot images directory still holds installer images from an
  earlier console or build, and a board is built with the installer product
- **THEN** the staged artifacts include only the installer image produced by
  this build

### Requirement: Update product emits signed bundles only

When the build is invoked with the update product, the build SHALL disable
installer media and full bootable disk image assembly by default, and SHALL
emit signed RAUC update bundles for both slots (unless a slot filter is set)
using the effective signing material.

#### Scenario: Update product skips installer and disk image

- **WHEN** a board is built with the update product and no explicit disk or
  installer overrides
- **THEN** the staged artifacts include signed `.raucb` bundles and do not
  include installer media or `disk-*.img`

#### Scenario: Explicit override can re-enable disk or installer

- **WHEN** a board is built with the update product and an explicit environment
  override re-enables disk or installer assembly
- **THEN** the override takes effect

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

### Requirement: Build-time SSH authorized key seed

The build SHALL accept an optional public SSH key for the `obake` login user
via the `OBKA_SSH_AUTHORIZED_KEY` environment variable, whose value names a
public key file. The build SHALL read that file regardless of where it resides
on the build host, including outside the build tree, and SHALL behave the same
on macOS and Linux build hosts. When set to a readable file, the key SHALL be
embedded in the installer medium as a seed that the installer provisions for
the `obake` user, and the image SHALL NOT provision a permanently honored key
outside that user's home. When unset or empty, no key seed SHALL be embedded.
When set to a path that is not a readable file, the build SHALL fail with an
error naming `OBKA_SSH_AUTHORIZED_KEY`.

#### Scenario: Key outside the build tree is honored

- **WHEN** an image is built with `OBKA_SSH_AUTHORIZED_KEY` naming a readable
  public key file that resides outside the repository
- **THEN** the build succeeds and the key is embedded in the installer medium
  as the `obake` seed

#### Scenario: Build host platform does not change the result

- **WHEN** the same image is built from a macOS host and from a Linux host with
  the same key input
- **THEN** both builds succeed and embed the same key seed

#### Scenario: Unreadable key path fails the build

- **WHEN** an image is built with `OBKA_SSH_AUTHORIZED_KEY` naming a path that
  is not a readable file
- **THEN** the build fails with an error naming `OBKA_SSH_AUTHORIZED_KEY`

#### Scenario: Unset key embeds no seed

- **WHEN** an image is built without `OBKA_SSH_AUTHORIZED_KEY` set
- **THEN** the installer medium carries no authorized key seed

### Requirement: Optional build-time obake password

The build SHALL accept an optional cleartext password for the `obake` login
account via the `OBKA_PASSWORD` environment variable. When unset or empty, the
account SHALL have no password and SSH password authentication as `obake`
SHALL fail. When set, the built image SHALL accept that password for local
console login and SHALL allow SSH password authentication as `obake` with that
password. The build-time password hash SHALL be written to the image rootfs
and SHALL NOT be seeded onto the persist partition; activating a different
slot's image replaces the seed password unless the `obake` user has already
changed it, in which case the user-set hash is retained by the persistent
`/etc` overlay.

#### Scenario: Default has no password

- **WHEN** an image is built without `OBKA_PASSWORD` set
- **THEN** the `obake` account has no usable password and SSH password
  authentication as `obake` fails

#### Scenario: Password set enables console and SSH password login

- **WHEN** an image is built with a non-empty `OBKA_PASSWORD`
- **THEN** the `obake` account accepts that password for local console login
  and an SSH client can authenticate as `obake` using that password

#### Scenario: Password is not persist state

- **WHEN** an image built with `OBKA_PASSWORD` is installed and the `obake`
  user has not changed the password
- **THEN** the build-time password hash is not seeded onto the persist
  partition and is replaced when a different slot image is activated

#### Scenario: Unchanged seed password is replaced on update

- **WHEN** a host that has not changed the seed password activates a different
  slot whose image was built with a different `OBKA_PASSWORD`
- **THEN** the `obake` account accepts the new image's password, not the old one

#### Scenario: User-changed password survives update

- **WHEN** the `obake` user changes its own password and the host later
  activates a different slot
- **THEN** the user-set password remains in effect and is not replaced by the
  slot image's seed

### Requirement: Build-time hostname seed

The build SHALL accept an optional host name for the installed host via the
`OBKA_HOSTNAME` environment variable. When set to a valid host name, the build
SHALL carry it as a seed that the installer medium provisions and that
`MAKE_DISK` images bake into their persist `/etc` overlay, and SHALL behave the
same on macOS and Linux build hosts. When unset or empty, the seed SHALL be the
default host name `obake`. When set to a value that is not a valid host name,
the build SHALL fail with an error naming `OBKA_HOSTNAME`.

#### Scenario: Seed carried by the installer medium

- **WHEN** an installer medium is built with `OBKA_HOSTNAME` set to a valid
  host name
- **THEN** the medium carries that name as the seed the installer provisions

#### Scenario: Seed baked into a disk image

- **WHEN** a disk image is built with `OBKA_HOSTNAME` set to a valid host name
- **THEN** the host boots with that name

#### Scenario: Invalid host name fails the build

- **WHEN** an image is built with `OBKA_HOSTNAME` set to a value that is not a
  valid host name
- **THEN** the build fails with an error naming `OBKA_HOSTNAME`

#### Scenario: Unset host name uses the default

- **WHEN** an image is built without `OBKA_HOSTNAME` set
- **THEN** the seed is the default host name `obake`

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

### Requirement: qdl image contains the linuxptp suite

The `qdl` image SHALL include the linuxptp 4.4 commands `ptp4l`,
`phc2sys`, `pmc`, `phc_ctl`, `hwstamp_ctl`, `timemaster`, `ts2phc`,
`nsm`, and `tz2alt`. Every other board image SHALL omit that suite.

#### Scenario: qdl rootfs contains the suite

- **WHEN** a `qdl` image is built
- **THEN** the root filesystem provides `ptp4l`, `phc2sys`, `pmc`,
  `phc_ctl`, `hwstamp_ctl`, `timemaster`, `ts2phc`, `nsm`, and `tz2alt`
  as executable commands

#### Scenario: Other boards omit the suite

- **WHEN** an image is built for a board other than `qdl`
- **THEN** the root filesystem does not provide `ptp4l`

### Requirement: qdl PTP services stay stopped until enabled

The `qdl` image SHALL provide `ptp4l` and `phc2sys` as system services
that are not enabled. An unmodified boot SHALL leave both stopped.
Starting or enabling `ptp4l` SHALL NOT start `phc2sys`.

#### Scenario: Fresh boot leaves both stopped

- **WHEN** a `qdl` host boots an image on which the operator has not
  enabled `ptp4l` or `phc2sys`
- **THEN** both services are inactive

#### Scenario: ptp4l does not start phc2sys

- **WHEN** the operator starts `ptp4l` and has not enabled `phc2sys`
- **THEN** `phc2sys` remains inactive

#### Scenario: Operator enables ptp4l

- **WHEN** the `obake` user enables and starts `ptp4l` over SSH
- **THEN** the action succeeds without a password prompt
- **AND** `journalctl -u ptp4l` shows that service's log

### Requirement: qdl PTP services run as the obake user

Started `ptp4l` and `phc2sys` services SHALL run as the `obake` user.
The running `ptp4l` service SHALL be able to bind the PTP event and
general ports and open the board PHC. The running `phc2sys` service
SHALL be able to discipline the system clock. Executing those binaries
from an `obake` login shell SHALL NOT grant that shell the same
privileges.

#### Scenario: Service identity is obake

- **WHEN** `ptp4l` is running as a system service
- **THEN** its process uid is `obake`
- **AND** it is not a service of the `obake` user manager

#### Scenario: Login shell does not inherit service privileges

- **WHEN** the `obake` user runs `ptp4l` from a login shell against the
  seeded configuration
- **THEN** the process cannot bind the PTP event port

### Requirement: obake can edit the qdl PTP configuration

The `ptp4l` service SHALL read its configuration from
`/home/obake/.config/linuxptp/`, and `phc2sys` SHALL read its options
from that same directory. The `obake` user SHALL be able to modify both
files. The image SHALL create a file there only when it is missing, and
SHALL leave an existing file unchanged across boot and image update.

#### Scenario: obake edits the ptp4l configuration

- **WHEN** the `obake` user changes
  `/home/obake/.config/linuxptp/ptp4l.cfg`
- **THEN** the write succeeds
- **AND** a later start of the `ptp4l` service uses that file

#### Scenario: Existing configuration is kept

- **WHEN** `/home/obake/.config/linuxptp/ptp4l.cfg` already exists and
  the host boots or applies an image update
- **THEN** the file contents are unchanged

### Requirement: seeded qdl PTP configuration is an AES67 client

The seeded `ptp4l` configuration SHALL be a client-only AES67/RAVENNA
profile: domain 0, end-to-end delay, UDPv4, and hardware timestamping.
It SHALL name no interface the image creates. The seeded `phc2sys`
options SHALL slew the system clock and SHALL NOT step it.

#### Scenario: Seed is a client with no real interface

- **WHEN** the seed creates `/home/obake/.config/linuxptp/ptp4l.cfg`
- **THEN** the file selects client-only operation, domain 0, end-to-end
  delay, UDPv4, and hardware timestamping
- **AND** it does not name an interface the image creates

#### Scenario: Seed does not step the system clock

- **WHEN** the seed creates the `phc2sys` options file
- **THEN** those options slew the system clock
- **AND** they do not request a step of the system clock

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
`ID` SHALL be `obake-os`; `VERSION` SHALL equal the contents of
`image/os/buildroot/VERSION` (or `OBAKE_IMAGE_VERSION` when set); `VERSION_ID`
SHALL be that version followed by `-g<shortsha>`, `-g<shortsha>-dirty`, or
`-gnogit`; and `PRETTY_NAME` SHALL be `obake-os` followed by `VERSION_ID`.
`<shortsha>` is the short git commit of the image tree. `-dirty` SHALL be
present when that tree has uncommitted changes. `-gnogit` SHALL be used when
the image git metadata cannot be read.

#### Scenario: os-release fields

- **WHEN** an image is built from a clean image git tree at commit `<shortsha>`
- **THEN** `/etc/os-release` has `NAME=obake-os`, `ID=obake-os`, `VERSION`
  matching the image version, `VERSION_ID` equal to `<version>-g<shortsha>`,
  and `PRETTY_NAME="obake-os <version>-g<shortsha>"`

#### Scenario: Aligns with image-version

- **WHEN** an image is built
- **THEN** os-release `VERSION` equals the contents of
  `/etc/obake/image-version`, and `VERSION_ID` begins with that same version
  followed by `-g`

#### Scenario: Dirty tree marks VERSION_ID

- **WHEN** an image is built from an image git tree with uncommitted changes
- **THEN** `VERSION_ID` ends with `-dirty` and `VERSION` equals
  `/etc/obake/image-version` with no dirty suffix

#### Scenario: Missing git metadata

- **WHEN** an image is built and the image git metadata cannot be read
- **THEN** `VERSION_ID` ends with `-gnogit` and `VERSION` equals
  `/etc/obake/image-version`

### Requirement: obake-os login banner

The built image SHALL display `Welcome to obake-os` as the login banner
(`/etc/issue`) instead of Buildroot's default welcome string.

#### Scenario: Login issue text

- **WHEN** an image is built
- **THEN** `/etc/issue` contains `Welcome to obake-os` and does not contain
  `Welcome to Buildroot`

### Requirement: Kernel release board and config revision

Each image's kernel release string (`uname -r`) SHALL end with
`-<rev>-<board>`, where `<rev>` is a positive integer when image git history
can be read, and `0` when it cannot. `<board>` is the board id. The
board id SHALL be `generic-x86_64`, `generic-aarch64`, `rk35xx`, or `qdl`
for those boards. The release string SHALL NOT contain the image git commit
and SHALL NOT end with `-dirty`.

#### Scenario: generic-x86_64 kernel release shape

- **WHEN** a `generic-x86_64` image is built from a clean image git tree
- **THEN** `uname -r` matches
  `<pinned-kernel-version>-<rev>-generic-x86_64` where `<rev>` is a positive
  integer

#### Scenario: generic-aarch64 kernel release shape

- **WHEN** a `generic-aarch64` image is built from a clean image git tree
- **THEN** `uname -r` matches
  `<pinned-kernel-version>-<rev>-generic-aarch64` where `<rev>` is a positive
  integer

#### Scenario: rk35xx kernel release shape

- **WHEN** an `rk35xx` image is built from a clean image git tree
- **THEN** `uname -r` matches `<pinned-kernel-version>-<rev>-rk35xx` where
  `<rev>` is a positive integer

#### Scenario: qdl kernel release shape

- **WHEN** a `qdl` image is built from a clean image git tree
- **THEN** `uname -r` matches `<pinned-kernel-version>-<rev>-qdl` where
  `<rev>` is a positive integer

#### Scenario: Release omits image git

- **WHEN** an image is built
- **THEN** `uname -r` does not contain an image git short commit and does not
  end with `-dirty`

#### Scenario: Missing git metadata uses rev 0

- **WHEN** an image is built and the image git metadata cannot be read
- **THEN** `uname -r` ends with `-0-<board>`

### Requirement: Kernel config revision from history

`<rev>` SHALL be the 1-based index of the board's current kernel-configuration
hash in the ordered set of distinct hashes of that board's kernel
configuration inputs across the image git history, oldest first. Inputs are
the board kernel configuration, the shared kernel fragments, and the board's
kernel config selection. The local-version suffix and the pinned kernel source
version are not inputs. The same hash SHALL select the same rev, including
when the configuration returns to an earlier hash. A commit that does not
change the hash SHALL NOT change `<rev>`.

#### Scenario: Unchanged config keeps the rev

- **WHEN** an image is rebuilt after a commit that does not change that
  board's kernel configuration inputs
- **THEN** `uname -r` is unchanged

#### Scenario: New config hash advances the rev

- **WHEN** a board's kernel configuration inputs change to a hash not yet
  present in the image git history and that change is committed
- **THEN** `<rev>` is one greater than the previous maximum rev for that board

#### Scenario: Reverted config reuses its rev

- **WHEN** a board's kernel configuration inputs return to a hash already
  present in the image git history
- **THEN** `<rev>` equals the rev previously assigned to that hash

#### Scenario: Kernel source pin does not bump the rev

- **WHEN** only the pinned kernel source version changes
- **THEN** `<rev>` is unchanged and the leading version in `uname -r` is the
  new pinned version

### Requirement: Dirty kernel config uses the next rev

When the image tree has uncommitted changes and the working-tree kernel
configuration hash has never been committed, the build SHALL use the next rev
for that build and SHALL NOT write that rev into the source tree. A dirty
tree whose hash is already in history SHALL use that hash's rev.

#### Scenario: Unseen dirty hash

- **WHEN** an image is built from a dirty image tree whose kernel
  configuration hash is not in git history
- **THEN** `<rev>` is one greater than the maximum committed rev for that
  board, and the build does not modify the source tree

#### Scenario: Dirty tree with a known hash

- **WHEN** an image is built from a dirty image tree whose kernel
  configuration hash matches one already in history
- **THEN** `<rev>` equals the rev of that hash

### Requirement: Shallow image git fails the build

The build SHALL fail when the image git checkout is shallow, so a truncated
history cannot assign a kernel rev.

#### Scenario: Shallow checkout

- **WHEN** an image build is started from a shallow image git checkout
- **THEN** the build fails before producing an image

### Requirement: Pinned inputs record the image version

The build SHALL record `image_version` in `pinned-inputs.json` using the same
image version that brands the rootfs and names update bundles:
`OBAKE_IMAGE_VERSION` when set, otherwise the version baked into the image
(or `image/os/buildroot/VERSION` when that is the source), defaulting to
`0.0.0` only when no version source is available.

#### Scenario: VERSION file propagates without override

- **WHEN** an image is built with `OBAKE_IMAGE_VERSION` unset and
  `image/os/buildroot/VERSION` containing a non-default version
- **THEN** `pinned-inputs.json` `image_version` equals that VERSION file
  contents and equals the artifact `VERSION` stamp

#### Scenario: Explicit override wins

- **WHEN** an image is built with `OBAKE_IMAGE_VERSION` set
- **THEN** `pinned-inputs.json` `image_version` equals
  `OBAKE_IMAGE_VERSION` and matches `/etc/obake/image-version` in the
  rootfs

### Requirement: Staged artifacts omit prior-version RAUC bundles

When a build emits signed RAUC update bundles for the current image version,
the Buildroot images tree and the staged artifact directory SHALL NOT retain
`obake-<other-version>-*.raucb` files from earlier builds of the same board
output.

#### Scenario: Version bump drops old bundles

- **WHEN** a board is rebuilt after the image version changes and RAUC signing
  material is configured
- **THEN** the staged artifact directory contains bundles named for the
  current version only and does not contain bundles named for the previous
  version

### Requirement: Singularity builds tolerate the default temporary directory

With no temporary-directory override, `singularity build --fakeroot` run by
`obake` SHALL complete a build from a local image, and SHALL NOT warn that
`nodev` is set on `/tmp`. The same absence of that warning SHALL hold for a
run that converts a non-local image into a SIF before starting it.

#### Scenario: Fakeroot build from a local image

- **WHEN** `obake` runs `singularity build --fakeroot` from a local image
  and does not set a temporary directory
- **THEN** the build completes and does not warn that `nodev` is set on
  `/tmp`

#### Scenario: Image conversion does not warn about nodev

- **WHEN** `obake` runs a container that Singularity must convert into a SIF
  first, without a temporary-directory override
- **THEN** the conversion does not warn that `nodev` is set on `/tmp`

### Requirement: qdl image contains obake-ravenna

The `qdl` image SHALL include the `obake-ravenna` program. Every
other board image SHALL omit that program.

#### Scenario: qdl rootfs contains the daemon

- **WHEN** a `qdl` image is built
- **THEN** the root filesystem provides `obake-ravenna` as an
  executable command

#### Scenario: Other boards omit the daemon

- **WHEN** an image is built for a board other than `qdl`
- **THEN** the root filesystem does not provide `obake-ravenna`

### Requirement: obake-ravenna is an enabled obake user service

The `qdl` image SHALL enable `obake-ravenna` as a user service of
the `obake` account so the service is active after boot without an
interactive login. It SHALL run in the `obake` user manager. It SHALL
NOT be installed as a system service.

#### Scenario: The user service is active after boot

- **WHEN** a `qdl` host has finished boot and no one has logged in
- **THEN** `obake-ravenna` is active in the `obake` user manager

#### Scenario: It is not a system service

- **WHEN** the system service manager on a `qdl` image is inspected
- **THEN** `obake-ravenna` is not installed as a system service

### Requirement: obake-ravenna configuration is seeded once

The `qdl` image SHALL create
`/home/obake/.config/obake-ravenna/` only when that configuration
is missing. The created file SHALL be owned by `obake` and SHALL
define no sources and no sinks. An existing configuration SHALL be
left unchanged across boot and image update.

#### Scenario: First boot seeds an empty stream list

- **WHEN** the configuration is absent and a `qdl` host boots
- **THEN** the configuration exists, is owned by `obake`, and defines
  no sources and no sinks

#### Scenario: An existing configuration is kept

- **WHEN** `/home/obake/.config/obake-ravenna/` already contains a
  configuration and the host boots or applies an image update
- **THEN** that configuration is unchanged

### Requirement: seeded ptp4l exposes a management socket to obake

The seeded `ptp4l` configuration SHALL place its management socket on
a path the `obake` user can open. The `ptp4l` service SHALL create the
directory for that socket as `obake`. This SHALL NOT enable `ptp4l`
at boot.

#### Scenario: A started seed is readable by obake

- **WHEN** `ptp4l` is started from a configuration the seed created
- **THEN** uid `obake` can open the management socket

#### Scenario: The socket change does not enable ptp4l

- **WHEN** a `qdl` host boots an image on which the operator has not
  enabled `ptp4l`
- **THEN** `ptp4l` is inactive
