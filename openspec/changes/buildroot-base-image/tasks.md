# Tasks

Shapes are out of scope for this change; no `shapes/*` files are touched, so no
shape/app versions are bumped. The affected host material is `image/os`,
`image/verification`, `image/installer`, `image/host`, and `image/docs`.

## 1. Buildroot base and cross toolchain

- [x] 1.1 Create the `BR2_EXTERNAL` tree under `image/os/` (`external.desc`,
  `Config.in`, `configs/`) and pin the Buildroot release; verify `make` with the
  base defconfig produces a minimal rootfs.
- [x] 1.2 Build the base rootfs for x86_64 and aarch64 from the same source;
  verify both boot to a running state under QEMU.
- [x] 1.3 Vendor the pinned inputs (the Buildroot release tarball and the
  complete `DL_DIR`, including the kernel tarball) under
  `image/os/buildroot/vendor/` with git-lfs via `vendor.sh`; verify a build that
  uses the vendor directory succeeds with networking disabled.
- [x] 1.4 Document the Buildroot layout, pins, and build command in
  `image/os/README.md`; verify the documented command rebuilds the base rootfs.

## 2. Custom realtime kernel

- [x] 2.1 Configure the `linux` package pinned to 7.2.9 with its source hash and
  a realtime config fragment; verify the built kernel enables
  `CONFIG_PREEMPT_RT` and boots.
- [x] 2.2 Write the kernel version into the image and tie it to the image
  version; verify the booted image reports the pinned realtime kernel and image
  version.
- [x] 2.3 Make the build fail when the realtime configuration is absent; verify
  a deliberate `BREAK`-style removal fails the build.

## 3. Minimal systemd base userland and host configuration

- [x] 3.1 Enable `BR2_INIT_SYSTEMD` and the guaranteed tool set in the base
  defconfig; verify the booted image provides `bash`, `grep`, `git`, `vi`,
  `sudo`, `ip`, `systemctl`, `journalctl`, and `dmesg`.
- [x] 3.2 Port `image/host/udev/*.rules` and `image/host/jack/*` plus the former
  tuning-layer files into a `BR2_EXTERNAL` package or rootfs overlay; verify
  they land under `/etc/udev/rules.d` and `/etc/jack`.
- [x] 3.3 Port the obake user and privileged access (polkit rule,
  `kernel.dmesg_restrict`, `systemd-journal`); verify `qemu/ssh-test.sh` passes.
  The privilege behavior is verified directly (SSH key login, polkit
  `systemctl`, `dmesg`, full journal, polkit `poweroff`); the `ssh-test.sh`
  harness itself also asserts the writable contract and `obake-launcher`, so it
  runs once the writable-contract and OTA work in section 7 lands.

## 4. Runtime layer (shapes runtime)

- [x] 4.1 Add a `BR2_EXTERNAL` meta-package `obake-runtime` whose `Config.in`
  `select`s `obake-jack2`, `alsa-utils`, `usb-modeswitch`, `nftables`, `avahi`
  (+ daemon), `nss-mdns`, `libcurl` (+ the `curl` binary), `iproute2`, `sudo`,
  and `git`; enable `BR2_TOOLCHAIN_BUILDROOT_CXX` and select the layer in the
  base defconfigs; verify the booted image provides `jackd` and `curl`.
- [x] 4.2 Verify the host JACK profiles and `jack@.service` from `image/host`
  run against the runtime `jack2`; verify the `/etc/jack` profiles are present
  and `jackd --version` works on the image.
- [x] 4.3 Document the runtime layer, its membership, and the dropped `rtkit` in
  `image/os/README.md`; verify the documented set matches the image.
- [x] 4.4 Add the external `obake-jack2` recipe (`--alsa --classic --dbus`) that
  depends on `alsa-lib`/`dbus`/`expat`, installs `jackd` + `jackdbus`, removes
  `jack_control`, and selects no Python; point `obake-runtime` at it and verify
  the booted image provides `jackdbus` with no `python3` or `jack_control`
  present, and that `jackdbus` acquires the `org.jackaudio.service` name on a
  D-Bus bus (jack2 ships it as a session-bus activation file).
- [x] 4.5 Provide the session D-Bus the runtime uses: ship the standard systemd
  user `dbus.socket`/`dbus.service` units, enable the socket for the `obake`
  user, and start that user manager at boot (`obake-linger.service`); verify
  `/run/user/<uid>/bus` exists after boot and that `jackdbus` is activatable as
  `org.jackaudio.service` on it.

## 5. Singularity CE runtime

- [x] 5.1 Add `package/singularity/` with a pinned tarball (4.5.1, since 4.6.0
  needs Go >= 1.26.8 and Buildroot ships 1.26.3) and SHA-256, build it with the
  cross-toolchain, and select it from an `obake-container` meta-package that
  `obake-runtime` selects; verify `singularity version` in the image matches the
  pin.
- [x] 5.2 Verify the runtime executes a container non-interactively on the
  booted image without a package manager present.

## 6. Per-slot unified kernel images and artifacts

- [x] 6.1 Add a post-image step invoking `ukify` to emit `obake-<slot>.efi`; verify
  the UKI filename encodes its slot and embeds the expected cmdline.
- [x] 6.2 Emit `root-<slot>.img`/`.tar`, `disk-<slot>.img`, `VERSION`,
  `SHA256SUMS`, and `image-manifest.json` from `partition-layout.json`; verify the
  disk image boots the active slot under QEMU.
- [x] 6.3 Reuse `scripts/sign.sh` and `scripts/verify-signature.sh`; verify the
  signature validates a good artifact and rejects a tampered one.

## 7. Writable contract, OTA, and installer

- [x] 7.1 Re-host the `/etc` overlay hook in the Buildroot initramfs; verify the
  overlay persists across a reboot and a slot switch and that `BREAK=etc` fails
  the check.
- [x] 7.2 Port `/var` tmpfs with its persisted subset, the persist and user
  partitions, and `tmpfiles.d`; verify `qemu/persist-test.sh` passes and
  `BREAK=var`/`BREAK=user` fail, and update `image/docs/host-config.md`.
- [x] 7.3 Wire `rauc/system.conf`, `obake-efi-entries.service`, the health gate,
  and `obake-update` onto the new base; verify `qemu/update-test.sh` and
  `qemu/health-test.sh` pass and `image/docs/os-update.md` matches.
- [x] 7.4 Rebuild the installer from the Buildroot base; verify
  `qemu/install-test.sh`, `qemu/install-interactive-test.sh`, and
  `qemu/reinstall-test.sh` pass, and update `image/installer/README.md`.

## 8. Build and verification decoupling

- [x] 8.1 Make the build container native to the host architecture and remove
  QEMU and OVMF from it; verify a build emits artifacts with no emulator
  present.
- [x] 8.2 Make verification consume a prebuilt artifact directory without
  building; verify `qemu/boot-test.sh` and `qemu/verify-test.sh` run against
  artifacts produced on a different host.
- [x] 8.3 Add the macOS arm64 aarch64 proxy loop (QEMU `virt` + `edk2-aarch64` +
  `hvf`); verify an aarch64 image boots accelerated and record its duration.
- [x] 8.4 Document the build/verify split and proxy loop in
  `image/verification/README.md`; verify the documented commands reproduce a
  passing run.

## 9. Board support

- [ ] 9.1 Add `generic` and `rk35xx` board defconfigs over the shared base;
  verify both images derive from the same base and differ only in the board
  overlay.
- [ ] 9.2 Add the RK35xx DTB and bootloader configuration; verify a board image
  builds and record its boot path.

## 10. Deterministic inputs and reproducibility

- [ ] 10.1 Emit the pinned-input set (Buildroot release, package versions,
  kernel, runtime); verify a same-host rebuild from unchanged pins yields a
  reproducible rootfs archive.
- [ ] 10.2 Update `verification/reproducibility.sh` for the new artifacts and
  verify two rebuilds with frozen caches select identical package, kernel, and
  runtime versions.

## 11. Integration verification

- [ ] 11.1 On x86_64, run install, customize, update, forced failed update, and
  revert via `qemu/integration-test.sh`; verify reversion and an unchanged user
  partition.
- [ ] 11.2 On the macOS arm64 aarch64 proxy, run the same integration flow;
  verify it behaves as on x86_64.
- [ ] 11.3 Retire the Debian `image/os` build and verify no `image/os` path still
  references the Debian layer model.
