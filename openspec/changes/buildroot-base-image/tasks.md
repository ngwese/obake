# Tasks

Shapes are out of scope for this change; no `shapes/*` files are touched, so no
shape/app versions are bumped. The affected host material is `image/os`,
`image/verification`, `image/installer`, `image/host`, and `image/docs`.

## 1. Buildroot base and cross toolchain

- [ ] 1.1 Create the `BR2_EXTERNAL` tree under `image/os/` (`external.desc`,
  `Config.in`, `configs/`) and pin the Buildroot release; verify `make` with the
  base defconfig produces a minimal rootfs.
- [ ] 1.2 Build the base rootfs for x86_64 and aarch64 from the same source;
  verify both boot to a running state under QEMU.
- [ ] 1.3 Vendor the Buildroot `DL_DIR` for the pinned release and build with
  networking disabled; verify the build succeeds offline.
- [ ] 1.4 Document the Buildroot layout, pins, and build command in
  `image/os/README.md`; verify the documented command rebuilds the base rootfs.

## 2. Custom realtime kernel

- [ ] 2.1 Configure the `linux` package with a pinned version and source hash
  plus a realtime config fragment; verify the built kernel enables
  `CONFIG_PREEMPT_RT` and boots.
- [ ] 2.2 Write the kernel version into the image and tie it to the image
  version; verify the booted image reports the pinned realtime kernel and image
  version.
- [ ] 2.3 Make the build fail when the realtime configuration is absent; verify
  a deliberate `BREAK`-style removal fails the build.

## 3. Minimal systemd base userland and host configuration

- [ ] 3.1 Enable `BR2_INIT_SYSTEMD` and the guaranteed tool set in the base
  defconfig; verify the booted image provides `bash`, `grep`, `git`, `vi`,
  `sudo`, `ip`, `systemctl`, `journalctl`, and `dmesg`.
- [ ] 3.2 Port `image/host/udev/*.rules` and `image/host/jack/*` plus the former
  tuning-layer files into a `BR2_EXTERNAL` package or rootfs overlay; verify
  they land under `/etc/udev/rules.d` and `/etc/jack`.
- [ ] 3.3 Port the obake user and privileged access (polkit rule,
  `kernel.dmesg_restrict`, `systemd-journal`); verify `qemu/ssh-test.sh` passes.

## 4. Singularity CE runtime

- [ ] 4.1 Add `package/singularity/` with a pinned tarball and SHA-256 and build
  it with the cross-toolchain; verify `singularity version` in the image matches
  the pin.
- [ ] 4.2 Verify the runtime executes a container non-interactively on the
  booted image without a package manager present.

## 5. Per-slot unified kernel images and artifacts

- [ ] 5.1 Add a post-image step invoking `ukify` to emit `obake-<slot>.efi`; verify
  the UKI filename encodes its slot and embeds the expected cmdline.
- [ ] 5.2 Emit `root-<slot>.img`/`.tar`, `disk-<slot>.img`, `VERSION`,
  `SHA256SUMS`, and `image-manifest.json` from `partition-layout.json`; verify the
  disk image boots the active slot under QEMU.
- [ ] 5.3 Reuse `scripts/sign.sh` and `scripts/verify-signature.sh`; verify the
  signature validates a good artifact and rejects a tampered one.

## 6. Writable contract, OTA, and installer

- [ ] 6.1 Re-host the `/etc` overlay hook in the Buildroot initramfs; verify the
  overlay persists across a reboot and a slot switch and that `BREAK=etc` fails
  the check.
- [ ] 6.2 Port `/var` tmpfs with its persisted subset, the persist and user
  partitions, and `tmpfiles.d`; verify `qemu/persist-test.sh` passes and
  `BREAK=var`/`BREAK=user` fail, and update `image/docs/host-config.md`.
- [ ] 6.3 Wire `rauc/system.conf`, `obake-efi-entries.service`, the health gate,
  and `obake-update` onto the new base; verify `qemu/update-test.sh` and
  `qemu/health-test.sh` pass and `image/docs/os-update.md` matches.
- [ ] 6.4 Rebuild the installer from the Buildroot base; verify
  `qemu/install-test.sh`, `qemu/install-interactive-test.sh`, and
  `qemu/reinstall-test.sh` pass, and update `image/installer/README.md`.

## 7. Build and verification decoupling

- [ ] 7.1 Make the build container native to the host architecture and remove
  QEMU and OVMF from it; verify a build emits artifacts with no emulator
  present.
- [ ] 7.2 Make verification consume a prebuilt artifact directory without
  building; verify `qemu/boot-test.sh` and `qemu/verify-test.sh` run against
  artifacts produced on a different host.
- [ ] 7.3 Add the macOS arm64 aarch64 proxy loop (QEMU `virt` + `edk2-aarch64` +
  `hvf`); verify an aarch64 image boots accelerated and record its duration.
- [ ] 7.4 Document the build/verify split and proxy loop in
  `image/verification/README.md`; verify the documented commands reproduce a
  passing run.

## 8. Board support

- [ ] 8.1 Add `generic` and `rk35xx` board defconfigs over the shared base;
  verify both images derive from the same base and differ only in the board
  overlay.
- [ ] 8.2 Add the RK35xx DTB and bootloader configuration; verify a board image
  builds and record its boot path.

## 9. Deterministic inputs and reproducibility

- [ ] 9.1 Emit the pinned-input set (Buildroot release, package versions, kernel,
  runtime); verify a same-host rebuild from unchanged pins yields a reproducible
  rootfs archive.
- [ ] 9.2 Update `verification/reproducibility.sh` for the new artifacts and
  verify two rebuilds with frozen caches select identical package, kernel, and
  runtime versions.

## 10. Integration verification

- [ ] 10.1 On x86_64, run install, customize, update, forced failed update, and
  revert via `qemu/integration-test.sh`; verify reversion and an unchanged user
  partition.
- [ ] 10.2 On the macOS arm64 aarch64 proxy, run the same integration flow;
  verify it behaves as on x86_64.
- [ ] 10.3 Retire the Debian `image/os` build and verify no `image/os` path still
  references the Debian layer model.
