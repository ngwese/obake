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
- Build and verification coupled only by an artifact set.
- Retain the per-slot UKI + RAUC A/B model byte-for-byte where it already works.

**Non-Goals:**

- Changing the RAUC bundle format, slot model, or signing.
- Changing the writable contract or the partition layout.
- Bit-for-bit cross-host reproducibility (deterministic inputs are required;
  see Decisions).
- Supporting a runtime package manager or in-place userland upgrades outside
  the A/B image update.

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

### Layer model expressed as BR2_EXTERNAL + defconfigs

The old L1..L5 layers become Buildroot idioms:

```
  old layer            Buildroot expression
  ---------            ------------------------------------------------
  base (trixie)        pinned Buildroot release + base defconfig
  kernel               linux package: version, hash, config fragment
  tuning               BR2_EXTERNAL package(s) + rootfs overlay
  runtime              BR2_EXTERNAL package: singularity
  board                board defconfig + BR2_EXTERNAL board directory
```

This is a real trade-off: Buildroot captures a whole selection in one defconfig
and `BR2_EXTERNAL` is a single external tree, so layers are no longer
independently swappable the way `layers/*/apply.sh` allowed. Bumping the
Buildroot release can require recipe fixes. Accepted because the goal is a
stable shared base, not per-layer rebasing.

### Init and userland: systemd, minimal set

Keep `BR2_INIT_SYSTEMD`. The host contract is systemd-shaped: `obake-runtime.target`,
`systemd-sysusers`/`sysusers.d`, `tmpfiles.d`, polkit rules, RAUC's D-Bus
service, and the health-gate units. The former tuning-layer packages fold into
this base userland. The guaranteed tool set is `bash`, `grep`, `git`, `vi`,
`sudo`, `ip`, `systemctl`, `journalctl`, `dmesg`, `singularity`, plus the
network/SSH pieces the contract already implies (`openssh`, `curl`, `avahi`).

- Alternative: BusyBox init (rejected: would require rewriting the tuning
  layer, RAUC D-Bus integration, and the privilege model for negligible size
  gain on a server image).

### Custom realtime kernel

Kernel is the standard `linux` package with a pinned version and hash, plus a
`BR2_LINUX_KERNEL_CONFIG_FRAGMENT_FILES` fragment enabling realtime
(`CONFIG_PREEMPT_RT`, plus the latency/audio options the project wants). Since
PREEMPT_RT merged mainline in 6.12, a fragment is sufficient; this completes
the task 9.1 work that was deferred. The kernel version is written to the image
and reported, tied to the image version.

### Singularity CE as a native in-tree package

Add `package/singularity/` under `BR2_EXTERNAL`: a pinned source tarball plus
SHA-256, built from source with the cross-toolchain. This keeps the "built from
source" property the spec requires and removes the staged-blob copy step.

This is the highest-risk task because Singularity CE is C + Go with a wide
dependency tree (libseccomp, squashfs, cryptsetup, FUSE, gpgme, glib). The
planned fallback, if some dependency cannot be cross-built cleanly, is a
pinned prebuilt staged tree installed by a package (which would require a spec
change); we do not take that fallback unless forced.

### Boot model: per-slot UKI retained

Retain the per-slot unified kernel image and RAUC's `efi` bootloader backend.
Buildroot produces `vmlinuz` and the initramfs; a post-image step invokes
`ukify` with the systemd EFI stub (from the host) to emit `obake-<slot>.efi`,
matching the current naming. `rauc/system.conf`, `obake-efi-entries.service`,
and the slot layout are unchanged.

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
airgapped and stable over time, Buildroot's `DL_DIR` (downloaded sources) is
vendored per release rather than fetched live.

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

- Exact Buildroot release and kernel version to pin (settled at
  implementation; does not change the approach).
- Whether the RK35xx defconfig uses mainline or vendor U-Boot and which DTB
  (settled when the board defconfig is written).
- Where the vendored `DL_DIR` lives (git-lfs versus a release artifact store).
