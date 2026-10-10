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

After a successful boot, the verification SHALL check core host behavior as
the `obake` login user, including the read-only root and the login-user
writable contract (`/home` writable; `/etc` and root writes fail), observe
`/etc` overlay topology without writing `/etc` as `obake`, and SHALL exercise
an update apply and revert through `obake-update`.

#### Scenario: Core host behavior checked

- **WHEN** the installed host has booted in QEMU
- **THEN** the harness confirms the root is read-only, the login-user
  writable contract behaves as specified, and an update applies and reverts
  through `obake-update` as specified

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

Every host-visible build or verification output directory SHALL live inside the
image repository, SHALL have a name beginning with an underscore, and SHALL be
ignored by version control. This includes staged build artifacts and harness
work or log trees produced by verification and developer-VM tooling.

#### Scenario: Outputs inside the repository

- **WHEN** the build runs from a clone of the image repository alone
- **THEN** every output directory is created inside that repository tree

#### Scenario: Underscore directories ignored

- **WHEN** output directories have been created by a build
- **THEN** version control ignores them

#### Scenario: Harness work and logs under underscore directories

- **WHEN** a verification harness runs with default output paths
- **THEN** its host-visible work directory and log files are created under an
  underscore-named directory inside the image repository

### Requirement: Host-owned build outputs

Files written from the build, verification, or developer-VM containers to the
host SHALL be owned by the user who invoked the tooling, so the user can read,
move, and remove them without elevated privileges. This includes staged build
artifacts and host-visible trees under the image repository's underscore
output directories used for verification and developer-VM work. The tooling
SHALL also reclaim known leftover foreign-owned verification scratch that
predated underscore defaults so the invoking user can remove it without
elevated privileges.

#### Scenario: Container outputs owned by the invoking user

- **WHEN** a containerized build completes
- **THEN** the staged files on the host are owned by the invoking user

#### Scenario: Cleanup needs no elevated privileges

- **WHEN** the invoking user removes a completed build's, verification run's,
  or developer-VM output directory
- **THEN** the removal succeeds without elevated privileges

#### Scenario: Containerized verification outputs owned by the invoking user

- **WHEN** a harness runs through the verification environment container and
  writes under the verification or developer-VM output directories
- **THEN** those host-visible files are owned by the invoking user

#### Scenario: Legacy harness scratch reclaimable without elevated privileges

- **WHEN** foreign-owned leftover files exist at known former harness default
  paths beside the verification scripts
- **THEN** the ownership reclaim path makes those files owned by the invoking
  user so removal succeeds without elevated privileges

### Requirement: Verification environment runs as the invoking user

The containerized verification environment SHALL run as the invoking user's
uid and gid, and when the host exposes `/dev/kvm` or `/var/run/docker.sock`
SHALL add the host groups that own those paths so accelerated QEMU and nested
build invocations keep working. Operator documentation SHALL state these host
access prerequisites for containerized verification, including the Docker
Buildx CLI plugin required to build the environment images.

#### Scenario: Verify-env identity matches the invoker

- **WHEN** an operator runs a harness through the verification environment
  wrapper
- **THEN** processes inside the container run as the invoker's uid and gid

#### Scenario: KVM group added when the device exists

- **WHEN** `/dev/kvm` exists on the host and the verification environment
  wrapper starts
- **THEN** the container is granted the host group that owns `/dev/kvm`

#### Scenario: Docker socket group added when the socket exists

- **WHEN** `/var/run/docker.sock` exists on the host and the verification
  environment wrapper starts
- **THEN** the container is granted the host group that owns the socket

#### Scenario: Prerequisites documented

- **WHEN** an operator reads the verification or developer QEMU documentation
- **THEN** the docs state that containerized verification needs the Docker
  Buildx CLI plugin, access to the Docker socket, and, for acceleration, to
  `/dev/kvm`

### Requirement: Environment images built with Docker Buildx

The build-environment and verification-environment wrappers SHALL build their
container images with Docker Buildx (BuildKit). When the Buildx CLI plugin is
unavailable they SHALL fail immediately with an error that names the missing
dependency and how to install it, and SHALL NOT fall back to the legacy
Docker builder.

#### Scenario: Env image build uses Buildx

- **WHEN** an operator invokes the build-environment or verification-
  environment wrapper and the env image must be (re)built
- **THEN** the wrapper builds that image through Docker Buildx

#### Scenario: Missing Buildx fails loudly

- **WHEN** the Buildx CLI plugin is not available on the host
- **THEN** the wrapper exits non-zero with a message that identifies the
  missing Buildx dependency before attempting a legacy `docker build`

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
performing a build. While copying artifacts out of the build cache and while
removing derived images from that cache, the build SHALL print a progress line
before each of those steps so the wait is visible.

#### Scenario: Locations printed

- **WHEN** a build completes
- **THEN** it prints the artifact directory and the produced artifact paths

#### Scenario: Directory reported without building

- **WHEN** the artifact directory is requested without building
- **THEN** the build prints the path and does not run a build

#### Scenario: Staging and cache cleanup announce themselves

- **WHEN** a containerized build copies artifacts out of the cache volume or
  deletes derived images from that volume
- **THEN** a progress line is printed before that step starts

### Requirement: Build product selection

The build entry point SHALL require exactly one product flag, `--installer` or
`--update`, when performing a build. Omitting both or passing both SHALL fail
with a usage error. Path discovery without building (`--print-dir`) SHALL NOT
require a product flag. Board profiles (`default` / `virtual`) SHALL continue to
supply installer console and target-disk defaults independently of the product
flag.

#### Scenario: Installer product selected

- **WHEN** an operator builds a board with `--installer`
- **THEN** the build emits installer media according to the board profile and
  any explicit installer overrides, and does not emit `.raucb` bundles unless
  bundle generation is explicitly requested

#### Scenario: Update product selected

- **WHEN** an operator builds a board with `--update`
- **THEN** the build emits signed RAUC update bundles and does not emit
  installer media or a full bootable disk image unless explicitly overridden

#### Scenario: Missing product flag rejected

- **WHEN** an operator invokes the build entry point without `--installer` or
  `--update`
- **THEN** the build fails with a usage error naming the required flags

#### Scenario: Both product flags rejected

- **WHEN** an operator passes both `--installer` and `--update`
- **THEN** the build fails with a usage error

#### Scenario: Print-dir needs no product flag

- **WHEN** an operator asks the build entry point only for the artifact
  directory path
- **THEN** the path is printed without requiring `--installer` or `--update`

### Requirement: Default development signing material

When signing certificate, key, or keyring paths are unset, the build entry
point SHALL use development signing material under the image repository's
`_build/signing` directory, creating that material if it is missing, so a
board installed from that host accepts update bundles later produced on the
same host without manually supplying keys. Explicit signing environment
variables SHALL override the defaults.

#### Scenario: Update build creates signing material when absent

- **WHEN** an operator builds with `--update` and no signing environment
  variables, and the default signing directory has no material
- **THEN** the build creates development signing material there and emits
  bundles signed with it

#### Scenario: Installer bakes the default keyring when unset

- **WHEN** an operator builds with `--installer` and no keyring environment
  variable
- **THEN** the built image trusts the default development CA from
  `_build/signing`

#### Scenario: Explicit signing overrides defaults

- **WHEN** an operator sets signing certificate, key, or keyring paths
  explicitly
- **THEN** the build uses those paths instead of the default development
  material

### Requirement: System CA trust store checked after boot

After a successful boot, the verification SHALL confirm that the installed
image exposes a system CA trust store usable by host TLS clients, so a
missing or empty store fails headless verification before release.

#### Scenario: CA trust store present in guest checks

- **WHEN** the installed host has booted in QEMU and guest functional checks
  run
- **THEN** the harness confirms the system CA trust store is present and
  non-empty

### Requirement: Root lock and update sudo checked after boot

After a successful boot, the verification SHALL confirm as `obake` that empty-
password `su` to root fails (proving the root account is locked without reading
`/etc/shadow`) and that `obake` may invoke both `obake-update` and a general
root shell via passwordless sudo.

#### Scenario: Root password field locked

- **WHEN** the installed host has booted in QEMU and guest functional checks
  run as `obake`
- **THEN** the harness confirms `su` to root with an empty password fails

#### Scenario: obake-update sudo allowed, shell sudo denied

- **WHEN** the installed host has booted in QEMU and guest functional checks
  run as `obake`
- **THEN** the harness confirms `sudo -n obake-update status` succeeds and
  `sudo -n /bin/sh -c true` succeeds

### Requirement: Guest functional checks run as obake

Guest functional check suites installed into verification images SHALL run as
the `obake` login user, using only privilege paths available to that user
(passwordless sudo for all commands, and polkit for systemd unit and power
operations). They SHALL NOT run as uid 0 and SHALL NOT invoke raw
`rauc install` as root.

#### Scenario: Verify service identity

- **WHEN** a `VERIFY=1` image boots and guest functional checks start
- **THEN** the check process runs as user `obake`

#### Scenario: Update checks use the client

- **WHEN** guest functional checks exercise bundle install, reject, or
  rollback
- **THEN** those actions go through `obake-update` rather than a direct root
  `rauc install`

### Requirement: Singularity run checked after boot

After a successful boot, the verification SHALL confirm that the `obake` user
can run a local Singularity SIF embedded in the verification image using the
setuid helper and kernel SquashFS mounts, so a FUSE-only or broken setuid
install fails headless verification before release.

#### Scenario: Local SIF run as obake

- **WHEN** the installed host has booted in QEMU and guest functional checks
  run
- **THEN** the harness confirms `singularity run` (or equivalent) of the
  embedded local SIF succeeds as `obake` and that the mount path is kernel
  SquashFS rather than squashfuse/FUSE

### Requirement: Singularity fakeroot checked after boot

After a successful boot, the verification SHALL confirm that the `obake` user
can use Singularity `--fakeroot` against the embedded local SIF, so missing
subordinate ID maps or a broken setuid fakeroot path fails headless
verification before release.

#### Scenario: Fakeroot exec as obake

- **WHEN** the installed host has booted in QEMU and guest functional checks
  run
- **THEN** the harness confirms `singularity exec --fakeroot` of the embedded
  local SIF reports uid `0` inside the container as `obake`

### Requirement: Guest checks for OS and kernel identity

After a successful boot, verification SHALL confirm that the guest presents
as `obake-os` in `/etc/os-release`, that the login banner is
`Welcome to obake-os`, and that `uname -r` includes the expected
`-g<shortsha>[-dirty]` provenance suffix (and the board LOCALVERSION suffix
when the board under test defines one).

#### Scenario: os-release and issue in guest

- **WHEN** the installed host has booted in QEMU and guest functional checks
  run
- **THEN** the checks confirm `NAME`/`ID` are `obake-os`, version fields match
  `/etc/obake/image-version`, and `/etc/issue` contains `Welcome to obake-os`

#### Scenario: Kernel release provenance in guest

- **WHEN** the installed host has booted in QEMU and guest functional checks
  run for a board that defines a LOCALVERSION suffix
- **THEN** the checks confirm `uname -r` contains that board suffix and a
  `-g` git shortsha segment

### Requirement: Writable media mount root checked after boot

After a successful boot, the verification SHALL confirm that `/media` is a
writable mount root suitable for removable-media automount (not merely an
empty directory on the read-only root), so a read-only `/media` fails
headless verification before release.

#### Scenario: Media root is writable

- **WHEN** guest functional checks run after boot
- **THEN** the suite fails unless `/media` is mounted and a privileged probe
  can create and remove a directory under `/media`

### Requirement: ESP access checked after boot

After a successful boot, the verification SHALL confirm that the `obake` user
can access `/boot/efi` without sudo, so an owner-only ESP mount fails headless
verification before release.

#### Scenario: Guest can access ESP as obake

- **WHEN** guest functional checks run after boot
- **THEN** the suite fails unless `obake` can list `/boot/efi` without sudo

### Requirement: Login accounting checked over SSH

After a successful SSH login as `obake` in QEMU, the verification SHALL confirm
that `last` runs without a missing-wtmp error and that its output includes the
`obake` user for the current session.

#### Scenario: last works after SSH login

- **WHEN** the harness has logged in over SSH as `obake` on the booted host
- **THEN** `last` succeeds and its output includes `obake`

### Requirement: Artifact pinned-inputs version matches VERSION

After a build stages artifacts, verification SHALL confirm that
`pinned-inputs.json` `image_version` equals the staged `VERSION` stamp, so a
mismatched image-version record fails headlessly without requiring a guest
boot.

#### Scenario: Matching version stamps

- **WHEN** a completed artifact directory is checked
- **THEN** `pinned-inputs.json` `image_version` equals the contents of
  `VERSION`

#### Scenario: Mismatch fails

- **WHEN** `pinned-inputs.json` `image_version` differs from `VERSION`
- **THEN** the check fails

### Requirement: Optional network cabling checked after boot

After a successful boot, the verification SHALL confirm that wired networkd
configuration does not require managed interfaces for online, so a wait-online
failure from missing carrier would be caught before release.

#### Scenario: Guest sees optional wired online requirement

- **WHEN** guest functional checks run after boot
- **THEN** the suite fails unless the active wired `.network` configuration
  marks managed interfaces as not required for online

### Requirement: User bus reachable over SSH

After a successful SSH login as `obake` in QEMU, the verification SHALL confirm
that `systemctl --user` can talk to the lingering user manager without the
harness exporting user-bus environment variables.

#### Scenario: systemctl --user works after SSH login

- **WHEN** the harness has logged in over SSH as `obake` on the booted host
- **THEN** `systemctl --user status` succeeds in that session without setting
  `XDG_RUNTIME_DIR` or `DBUS_SESSION_BUS_ADDRESS` in the harness command
