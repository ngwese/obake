# Tasks

Shapes are out of scope for this change; no `shapes/*` files are touched. The
`host/` udev rules and JACK configuration are consumed by the tuning layer.

## 1. Build environment and image pipeline

- [ ] 1.1 Create a pinned, containerized Linux build environment that runs on
  macOS arm64 and Windows 11 WSL x86_64; verify the build runs in the container
  on both hosts.
- [ ] 1.2 Verify the containerized environment produces equivalent x86_64 image
  artifacts on both hosts; compare artifacts built on macOS arm64 and WSL x86_64.
- [ ] 1.3 Create the layered build skeleton (base, kernel, tuning, runtime) using
  mmdebstrap on a pinned Debian trixie base plus a local apt repository; verify
  the base layer builds a minimal rootfs.
- [ ] 1.4 Pin the base distribution and all layer inputs in a version manifest;
  verify two clean builds of the base layer are byte-identical.
- [ ] 1.5 Package the custom realtime kernel as a Debian package; verify the
  image boots under qemu and reports the pinned realtime kernel version.
- [ ] 1.6 Move `host/udev/*.rules` and `host/jack/*.conf` plus
  `host/jack/jack@.service` into the tuning layer package; verify the built
  image places them under `/etc/udev/rules.d` and `/etc/jack`.
- [ ] 1.7 Add per-board tuning metapackages; verify two board images derive from
  one shared base and differ only in the tuning layer.
- [ ] 1.8 Package Apptainer built from source at a pinned version; verify
  `apptainer version` inside the image matches the pin.
- [ ] 1.9 Generate the per-slot unified kernel image with `ukify` (kernel,
  initrd, cmdline); verify `ukify` is present in the base and the UKI filename
  encodes its slot and embeds the expected cmdline.
- [ ] 1.10 Emit a versioned image artifact and a signature; verify the signature
  validates for a good artifact and fails for a tampered one.
- [ ] 1.11 Document the layer model and build commands in `os/README.md`; verify
  the documented commands rebuild an image as written.

## 2. Partition layout and installer

- [x] 2.1 Define the stable partition labels and GPT type GUIDs in one shared
  metadata file; verify both the build and installer read the same definitions.
- [ ] 2.2 Implement disk enumeration, target selection, and destructive-action
  confirmation over serial or text console; verify a headless run via
  `qemu -nographic` completes selection and confirmation.
- [ ] 2.3 Implement partitioning for ESP, root A, root B, persist, and user;
  verify created partitions carry the expected labels and GUIDs.
- [ ] 2.4 Install systemd-boot, both per-slot UKIs, and BLS entries; verify the
  first boot loads the active slot's UKI and root.
- [ ] 2.5 Seed the persist partition (machine-id, SSH host keys, hostname,
  network configuration) and create the user partition; verify the seeded
  contents after install.
- [ ] 2.6 Implement reinstall preserving the user partition; verify user data is
  unchanged after reinstalling over an existing layout.
- [ ] 2.7 Run the installer end to end under the QEMU harness on x86_64 and on
  arm64 hardware; verify both produce a bootable host.
- [ ] 2.8 Document installer usage and the stable layout in `installer/README.md`;
  verify the documented flow matches an actual install.

## 3. Read-only root and writable contract

- [ ] 3.1 Mount the root filesystem read-only at runtime; verify a write to a
  non-contract path fails.
- [ ] 3.2 Back an `/etc` overlay with the persist partition; verify a change
  under `/etc` survives a reboot and a slot switch.
- [ ] 3.3 Mount `/var` as tmpfs with a persisted subset on the persist
  partition; verify transient content is discarded and the subset is retained
  across a reboot.
- [ ] 3.4 Mount the user partition for user access; verify an update and a
  preserving reinstall leave it unmodified.
- [ ] 3.5 Define the directory contract and SSH access; verify a user can modify
  files within the contract and cannot modify the root outside it.
- [ ] 3.6 Document the writable contract and directory layout in
  `docs/host-config.md`; verify the documented paths match the running system.

## 4. RAUC A/B updates

- [ ] 4.1 Configure RAUC (`efi` bootloader, root slots, keyring); verify
  `rauc status` reports both slots.
- [ ] 4.2 Establish signing material and bundle signing; verify a valid bundle
  is accepted and an unsigned or tampered bundle is rejected with no slot
  changes.
- [ ] 4.3 Apply a bundle to the inactive slot and its matching UKI; verify only
  the inactive slot is written and the update activates on reboot.
- [ ] 4.4 Add USB and signed-HTTPS bundle sources with USB preferred; verify an
  airgapped USB update succeeds and that USB wins when both are available.
- [ ] 4.5 Gate updates on an explicit user request; verify no update occurs
  without one.
- [ ] 4.6 Expose update status; verify it reports running version, active slot,
  and pending or failed state.
- [ ] 4.7 Bump the pinned kernel and Apptainer versions and rebuild; verify the
  new versions are reported after installing the resulting bundle.
- [ ] 4.8 Document the update procedure and signing workflow in
  `docs/os-update.md`; verify the documented commands perform an update.

## 5. Health gate and rollback

- [ ] 5.1 Snapshot user configuration to the persist partition before applying
  an update; verify the snapshot exists before the inactive slot is written.
- [ ] 5.2 Implement the health-gate service to validate over a bounded number of
  boots, restore the prior configuration, and mark the slot good; verify a
  healthy boot marks the slot good.
- [ ] 5.3 Wire systemd-boot boot counting to auto-revert; verify a forced
  validation failure reverts to the previous slot and restores configuration.
- [ ] 5.4 Make the try count configurable; verify the configured value is
  honored.
- [ ] 5.5 Document the health-gate criteria and rollback behavior in
  `docs/os-update.md`; verify the described behavior matches a forced-failure
  run.

## 6. systemd-boot update path

- [ ] 6.1 Prototype a fwupd UEFI capsule carrying systemd-boot; verify the
  capsule applies and the host still boots.
- [ ] 6.2 Provide a `bootctl update` fallback path; verify the host boots after
  a boot-manager update.
- [ ] 6.3 Verify the boot-manager version stays consistent across slot switches.

## 7. QEMU verification harness

- [ ] 7.1 Build a headless harness that boots a built image under QEMU with OVMF
  UEFI firmware and asserts the system reaches a running state; verify it passes
  for a good image and fails for a deliberately broken one.
- [ ] 7.2 Extend the harness to run the installer against a virtual disk, boot
  the installed host, and assert it functions; verify a full install-then-boot
  run passes.
- [ ] 7.3 Add functional checks to the harness (read-only root, writable
  contract, update apply and revert); verify each check fails when its behavior
  is deliberately broken.
- [ ] 7.4 Verify the harness runs without a display and reports a deterministic
  pass or fail; verify it produces the same result on macOS arm64 and WSL x86_64.
- [ ] 7.5 Document how to run the build environment and QEMU harness in
  `verification/README.md`; verify the documented commands reproduce a passing
  run.

## 8. Integration verification

- [ ] 8.1 Using the QEMU harness on x86_64, run install -> customize -> update ->
  reboot, then force a failed update; verify headlessly that the system reverts
  and configuration is restored.
- [ ] 8.2 Repeat the full install and update cycle on arm64 hardware; verify it
  behaves as on x86_64.
- [ ] 8.3 Verify that a complete install, update, revert, and preserving
  reinstall never modify the user partition.
