# Design

## Context

See proposal.md for motivation. The current image build (`image/os`) is a
layered Debian build: `mmdebstrap` produces a minimal trixie rootfs, the kernel
is the stock `linux-image-rt-*` package, and tuning/runtime/board are expressed
as package lists plus `files/` overlays driven by `layers/*/apply.sh`. Every
input except the Singularity source is pinned only against a moving
`deb.debian.org` mirror, so rebuilds drift. The build environment
(`image/verification/build-env`) runs a `linux/amd64` container so that
`mmdebstrap` can run target-architecture maintainer scripts under qemu binfmt;
that is the only reason x86_64 images build on macOS arm64.

Material that survives the base swap largely unchanged: `partition-layout.json`,
`rauc/system.conf`, the installer partitioning logic, `host/udev/*.rules`,
`host/jack/*`, the health gate, `obake-launcher`, `obake-update`, and the QEMU
harnesses.

## Goals / Non-Goals

**Goals:**

- A single pinned build whose inputs do not drift, so a board change cannot
  move the shared base.
- A custom realtime kernel built from source as an ordinary build output.
- Cross-compilation with no target-architecture execution, so any Linux host
  (native arm64 container on macOS) builds any target.
- The shapes runtime as a distinct, reusable meta-package layer above the
  minimal base.
- Build and verification coupled only by an artifact set.
- Retain the per-slot UKI + RAUC A/B model byte-for-byte where it already works.

**Non-Goals:**

- Changing the RAUC bundle format, slot model, or signing.
- Changing the writable contract or the partition layout.
- Bit-for-bit cross-host reproducibility (deterministic inputs are required;
  see Decisions).
- Supporting a runtime package manager or in-place userland upgrades outside
  the A/B image update.
- Implementing the future developer-tooling or audio-over-IP meta-packages;
  this change establishes the runtime layer and the meta-package pattern only.

## Decisions

### Build basis: Buildroot over pinned Debian + snapshot

Replace `mmdebstrap` with a single pinned Buildroot tree under `BR2_EXTERNAL`.
Buildroot pins every package to an exact version and source hash inside a
pinned release, and it is a cross-compilation system: it does not run target
binaries, so the emulation dependency disappears by construction. This answers
the drift problem at its source rather than by pinning a mirror.

- Alternative: Debian via `snapshot.debian.org` date pin (rejected: keeps the
  emulation path and the maintainer-script execution model, and adds a
  third-party snapshot dependency).
- Alternative: Yocto (rejected: `meta-*` layers map well onto the old layer
  model and `meta-rauc` is mature, but the toolchain and build times are far
  heavier than this minimal system needs).
- Alternative: exact `pkg=version` pins on `deb.debian.org` (rejected:
  transitive dependencies still drift and the list needs constant upkeep).

### Layer model: minimal base + runtime layer + board

The old L1..L5 layers map to Buildroot idioms, with the shapes runtime
expressed as a named meta-package so it is one explicit layer above the minimal
base:

```
  old layer     Buildroot expression
  ---------     --------------------------------------------------------
  base          pinned Buildroot release + base defconfig + rootfs overlay
  kernel        linux package: version, hash, realtime config fragment
  runtime       BR2_EXTERNAL meta-package `obake-runtime` (+ singularity)
  board         board defconfig + BR2_EXTERNAL board directory
```

The **minimal base** is systemd, the admin tool set, and the host
configuration (users, udev, JACK profiles, polkit). The **runtime layer** is the
set the shapes build and run on: the audio and networking runtime plus the
Singularity container runtime. It is a single `BR2_EXTERNAL` meta-package whose
`Config.in` `select`s each member package, so a defconfig names the layer once
and future board defconfigs reuse it.

`obake-runtime` selects, at present: `obake-jack2` (jackd + jackdbus),
`alsa-utils`, `usb-modeswitch`, `nftables`, `avahi` (+ daemon), `nss-mdns`,
`libcurl` (+ the `curl` binary), `iproute2`, `sudo`, `git`, and
`obake-container`. `obake-jack2` needs `BR2_TOOLCHAIN_BUILDROOT_CXX`, enabled
in the base.

The **container runtime** is its own meta-package, `obake-container`
(`package/obake-container/`), because SingularityCE pulls a large dependency
tree (Go, gnupg2/gpgme, cryptsetup, FUSE3, seccomp, squashfs tools, …). Keeping
it a named meta-package, selected by `obake-runtime`, matches the meta-package
pattern and keeps the audio/network runtime separate from the container runtime.

`rtkit` is intentionally not included: Buildroot has no `rtkit` package, and
realtime scheduling and locked memory are already provided by the `limits.d`
grants and the PREEMPT_RT kernel.

Additional meta-packages are expected in future changes — developer tooling,
and audio-over-IP kernel modules plus userspace tooling. This change establishes
the runtime layer and the meta-package pattern and does not implement those.

Trade-off: Buildroot captures a whole selection in one defconfig and
`BR2_EXTERNAL` is a single external tree, so layers are not independently
swappable the way `layers/*/apply.sh` allowed. Bumping the Buildroot release can
require recipe fixes. Accepted because the goal is a stable shared base, not
per-layer rebasing.

### JACK runtime: external jack2 recipe without Python

`jackdbus` is C: `dbus/wscript` builds it from `jackdbus.c`, `controller*.c`,
`params.c`, and the `xml*.c` files, and its configure step checks only `dbus-1`
and `expat`. The Python dependency that Buildroot's in-tree
`BR2_PACKAGE_JACK2_DBUS` drags in (`python3`, `dbus-python`,
`python3-pyexpat`) exists solely for the `jack_control` helper, which the same
`wscript` installs unconditionally as a `#! /usr/bin/python3` script.

The runtime layer therefore uses an external recipe, `obake-jack2`
(`package/obake-jack2/`), that configures `--alsa --classic --dbus`, depends on
`alsa-lib`, `dbus`, and `expat`, installs both `jackd` and `jackdbus`, and
removes `jack_control` after install. No Python interpreter or bindings are
selected, so the host carries no Python runtime. `jackdbus` is driven over its
D-Bus interface (`org.jackaudio`).

- Alternative: the in-tree `BR2_PACKAGE_JACK2_DBUS` (rejected: selects a Python
  runtime and bindings for a convenience CLI nothing on the host uses, and then
  deletes that CLI anyway in the dbus-only case).

`jackdbus` is D-Bus activated, so the runtime also ships a session bus for the
`obake` user: the standard systemd user `dbus.socket`/`dbus.service` units
(Buildroot does not package them), enabled on the user manager, with the user
manager started at boot by `obake-linger.service` (enable-linger plus an
explicit `user@<uid>.service` start). `org.jackaudio.service` is then
activatable on `unix:path=/run/user/<uid>/bus`. The to-be-written
`obake-launcher` will use this bus to activate and configure `jackd` over
D-Bus. Because `/var` becomes tmpfs with a persisted subset later, the linger
state (`/var/lib/systemd/linger`) must live on that persisted subset.

### Init and base userland: systemd, minimal set

Keep `BR2_INIT_SYSTEMD`. The host contract is systemd-shaped: `obake-runtime.target`,
`systemd-sysusers`/`sysusers.d`, `tmpfiles.d`, polkit rules, RAUC's D-Bus
service, and the health-gate units. The minimal base userland provides `bash`,
`grep`, `vi`, `sudo`, `systemctl`, `journalctl`, `dmesg`, `openssh`, and polkit.
The runtime packages the shapes build on — including `obake-jack2` and the
container runtime — are a separate layer above this base (see the layer model
above).

- Alternative: BusyBox init (rejected: would require rewriting the tuning
  layer, RAUC D-Bus integration, and the privilege model for negligible size
  gain on a server image).

### Custom realtime kernel

Kernel is the standard `linux` package pinned to the latest stable release that
carries PREEMPT_RT (7.2.9), plus a
`BR2_LINUX_KERNEL_CONFIG_FRAGMENT_FILES` fragment enabling realtime
(`CONFIG_PREEMPT_RT`, plus the latency/audio options the project wants). Because
PREEMPT_RT has been mainline since 6.12, no out-of-tree RT patchset is needed
and a fragment is sufficient; this completes the task 9.1 work that was
deferred. The kernel version is written to the image and reported, tied to the
image version.

- Alternative: a longterm kernel (6.18.x or 6.12.x) for years of stable fixes
  (deferred: the latest stable is preferred for current driver/hardware
  support; switch to a longterm branch if long-lived fix support is required).

### Singularity CE as a native in-tree package

Add `package/singularity/` under `BR2_EXTERNAL`: a pinned source tarball plus
SHA-256, built from source with the cross-toolchain via its `mconfig`/`makeit`
build system, and its CGO code cross-built with the Buildroot Go toolchain
(`HOST_GO_TARGET_ENV`). This keeps the "built from source" property the spec
requires and removes the staged-blob copy step. The container runtime is
grouped under the `obake-container` meta-package (see the layer model above).

The pin is **SingularityCE 4.5.1**, not 4.6.0: 4.6.0's `go.mod` requires Go
>= 1.26.8, while Buildroot's newest Go is 1.26.6 (even in 2026.08) and our
pinned 2026.02.3 ships 1.26.3. 4.5.1 requires Go >= 1.25.9, which the pinned
toolchain satisfies. Revisit the pin when a Buildroot/Go bump provides 1.26.8+.

The runtime is configured `--without-suid` (the host runs as root) and
`--without-libsubid`. The board kernel config must provide loop devices
(`CONFIG_BLK_DEV_LOOP`) and SquashFS (`CONFIG_SQUASHFS` plus the decompressors)
so the runtime can mount SIF images. This is the highest-risk task because
Singularity CE is C + Go with a wide dependency tree (libseccomp, squashfs,
cryptsetup, FUSE, gpgme). The planned fallback, if some dependency cannot be
cross-built cleanly, is a pinned prebuilt staged tree installed by a package
(which would require a spec change); we do not take that fallback unless forced.

### Boot model: per-slot UKI retained

Retain the per-slot unified kernel image and RAUC's `efi` bootloader backend.
A post-image step (`board/obake/post-image.sh`) invokes `ukify` (from
Buildroot's `host-systemd`) to emit `obake-<slot>.efi`, then reuses
`scripts/make-disk.sh` and `scripts/sign.sh`. `rauc/system.conf`,
`obake-efi-entries.service`, and the slot layout are unchanged.

`ukify` needs a systemd EFI stub. Buildroot only builds a stub for x86 targets
(`BR2_PACKAGE_SYSTEMD_BOOT`), so the x86_64 defconfig enables it and uses the
in-tree stub, while the aarch64 stub is built once from the pinned systemd
source and vendored under `vendor/efi-stub/` (hashed in `pins.json`). The
initramfs is bundled once the writable-contract work provides it; until then
the UKI carries the kernel and command line only.

```
  ESP  /EFI/Linux/obake-a.efi   obake-b.efi     (per-slot UKIs)
  ROOT_A  OBKA_ROOT_A (ext4, ro)   ROOT_B  OBKA_ROOT_B (ext4, ro)
  PERSIST OBKA_PERSIST             USER    OBKA_USER
```

### Writable contract re-hosted on the Buildroot initramfs

The `/etc` overlay hook currently runs from `initramfs-tools`
(`local-bottom/obake-etc-overlay`). Buildroot does not use `initramfs-tools`,
so the hook is re-hosted in Buildroot's initramfs: mount `OBKA_PERSIST` by
label, mount the overlayfs `/etc` before `switch_root`, and fall back to the
read-only `/etc` if persist is unavailable. `/var` tmpfs with its persisted
subset, systemd `tmpfiles.d`, and the persist/user separation port unchanged.
`partition-layout.json` remains the single source of truth for the installer
and disk assembly.

### Build and verification decoupled by an artifact set

The build emits a self-contained artifact set (`root-<slot>.tar`/`.img`,
`obake-<slot>.efi`, `VERSION`, `SHA256SUMS`, `image-manifest.json`, signatures)
and nothing else. Verification consumes that directory; it does not build.

```
  BUILD HOST (any Linux, arm64-native container on macOS; no qemu/ovmf)
     build -> dist/os/<arch>/...
                    |
                    v
  VERIFY HOST (may differ)
     x86_64 : qemu + OVMF + kvm
     aarch64: qemu virt + edk2-aarch64 + hvf (macOS arm64)  <- proxy
```

The build container drops QEMU and OVMF entirely. On macOS arm64 the iteration
target becomes the aarch64 image, booted accelerated under `hvf`, as a proxy
for the real arm64 board. x86_64 verification falls back to a host that can
run it.

### Deterministic inputs, not bit-identical cross-host output

The requirement is deterministic inputs: same pins select the same package,
kernel, and runtime versions anywhere. Buildroot honors `SOURCE_DATE_EPOCH`,
and the canonical rootfs archive is reproducible for a given host and pin. We
do not require bit-identical output across different host architectures, and
the `os-image-build` deltas state intent accordingly. To keep builds working
airgapped and stable over time, the pinned inputs are vendored in the tree with
git-lfs.

### Pinned inputs vendored with git-lfs

The pinned inputs for a release — the Buildroot release tarball and the
complete Buildroot download cache (`DL_DIR`, which includes the pinned kernel
tarball) — are committed under `image/os/buildroot/vendor/` and tracked with
git-lfs, the same mechanism the repository already uses for shape tarballs.
`build.sh` uses them directly when present, so a release builds with no network
access, and `vendor.sh` repopulates them from a verified build when a pin
changes.

- Alternative: a separate release artifact store (rejected: adds infrastructure
  the project does not otherwise run; git-lfs keeps inputs in the repository
  with the pins that describe them).

## Risks / Trade-offs

- [Singularity CE dependency tree resists cross-build] -> Build it first as a
  spike; if a dependency blocks, fall back to a pinned prebuilt staged tree and
  amend the spec.
- [/etc overlay port is timing-sensitive] -> Port the existing shell logic
  as-is with explicit wait/retry on the persist device and an overlay module
  load; verify with the existing `persist-test.sh` and the `BREAK` fault
  injection.
- [A Buildroot package executes a target binary at build time] -> Prefer
  host-tool equivalents; if unavoidable, scope qemu-user to that package only
  rather than the whole build.
- [Buildroot release bump breaks recipes] -> Pin the release; bumping is a
  deliberate, tested change with no expectation of untouched higher layers.
- [arm64 QEMU `virt` + UEFI proxy diverges from RK35xx U-Boot] -> The proxy
  validates rootfs, RAUC, UKI, and the writable contract, not U-Boot/DTB;
  keep a hardware smoke test for the board's boot path.
- [Buildroot release predates the chosen kernel version] -> use Buildroot's
  custom kernel version + tarball/hash path rather than a known-version
  shortcut, and verify the toolchain headers build against it.
- [Live source downloads drift or are blocked] -> Vendor `DL_DIR` per release.
- [systemd base is larger than a BusyBox base] -> Accepted; the contract and
  RAUC integration depend on it.

## Migration Plan

Greenfield replacement; no running host is migrated.

1. Stand up `BR2_EXTERNAL` with the base defconfig, systemd, and userland;
   confirm a rootfs builds and boots under QEMU.
2. Port the kernel fragment and per-slot UKI generation; confirm the realtime
   kernel reports as tied to the image version.
3. Add the Singularity package; confirm `singularity version` matches the pin.
4. Port the tuning overlay and the `/etc` overlay into the Buildroot
   initramfs; confirm the writable contract via the persist checks.
5. Wire RAUC, signing, the health gate, and the installer onto the new base;
   confirm an A/B update applies and reverts.
6. Split the build container from verification; confirm a build with no QEMU
   and a verification run on a separate host, plus the macOS arm64 proxy loop.
7. Retire the Debian `image/os` build.

Rollback is reverting the branch; the Debian build remains until step 7.

## Open Questions

- Exact Buildroot release to pin (settled at implementation; does not change
  the approach).
- Whether the RK35xx defconfig uses mainline or vendor U-Boot and which DTB
  (settled when the board defconfig is written).
