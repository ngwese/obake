# Tasks

Shapes are out of scope for this change; no `shapes/*` files are touched. The
`host/` udev rules and JACK configuration are consumed by the tuning layer.

## 1. Build environment and image pipeline

- [ ] 1.1a Create a pinned, containerized Linux build environment and verify the
  build runs in the container on macOS arm64.
- [x] 1.1b Verify the build runs in the same containerized environment on
  Windows 11 WSL x86_64.
- [ ] 1.2a On macOS arm64, build the x86_64 image and record the artifact
  digests.
- [x] 1.2b On Windows 11 WSL x86_64, build the x86_64 image and record the
  artifact digests.
- [ ] 1.2c Compare the macOS arm64 and WSL x86_64 digests recorded by 1.2a/1.2b
  and confirm the artifacts are equivalent. Depends on 1.2a.
- [x] 1.3 Create the layered build skeleton (base, kernel, tuning, runtime)
  over mmdebstrap on a pinned Debian trixie base; verify the base layer builds
  a minimal rootfs.
- [x] 1.4 Pin the base distribution and all layer inputs in a version manifest;
  verify two clean builds of the base layer produce byte-identical rootfs
  archives and UKIs.
- [x] 1.5 Include a pinned realtime kernel (stock Debian PREEMPT_RT package for
  now; a fully custom kernel build is deferred to 9.1); verify the image boots
  under qemu and reports the pinned realtime kernel version.
- [x] 1.6 Move `host/udev/*.rules` and `host/jack/*.conf` plus
  `host/jack/jack@.service` into the tuning layer package; verify the built
  image places them under `/etc/udev/rules.d` and `/etc/jack`.
- [x] 1.7 Add per-board tuning metapackages; verify two board images derive from
  one shared base and differ only in the tuning layer.
- [x] 1.8 Package Singularity CE built from source at a pinned version; verify
  `singularity version` inside the image matches the pin.
- [x] 1.9 Generate the per-slot unified kernel image with `ukify` (kernel,
  initrd, cmdline); verify `ukify` is present in the base and the UKI filename
  encodes its slot and embeds the expected cmdline.
- [x] 1.10 Emit a versioned image artifact and a signature; verify the signature
  validates for a good artifact and fails for a tampered one.
- [x] 1.11 Document the layer model and build commands in `os/README.md`; verify
  the documented commands rebuild an image as written.

## 2. Partition layout and installer

- [x] 2.1 Define the stable partition labels and GPT type GUIDs in one shared
  metadata file; verify both the build and installer read the same definitions.
- [x] 2.2 Implement disk enumeration, target selection, and destructive-action
  confirmation over serial or text console; verify a headless run via
  `qemu -nographic` completes selection and confirmation.
- [x] 2.3 Implement partitioning for ESP, root A, root B, persist, and user;
  verify created partitions carry the expected labels and GUIDs.
- [x] 2.4 Install both per-slot UKIs on the ESP and create an EFI boot entry
  for each slot with the active slot first; verify the first boot loads the
  active slot's UKI and root.
- [x] 2.5 Seed the persist partition (machine-id, SSH host keys, hostname,
  network configuration) and create the user partition; verify the seeded
  contents after install.
- [x] 2.6 Implement reinstall preserving the user partition; verify user data is
  unchanged after reinstalling over an existing layout.
- [x] 2.7a Run the installer end to end under the QEMU harness on x86_64;
  verify it produces a bootable host.
- [ ] 2.7b Run the installer end to end on arm64 hardware; verify it produces a
  bootable host.
- [x] 2.8 Document installer usage and the stable layout in `installer/README.md`;
  verify the documented flow matches an actual install.

## 3. Read-only root and writable contract

- [x] 3.1 Mount the root filesystem read-only at runtime; verify a write to a
  non-contract path fails.
- [x] 3.2 Back an `/etc` overlay with the persist partition; verify a change
  under `/etc` survives a reboot and a slot switch.
- [x] 3.3 Mount `/var` as tmpfs with a persisted subset on the persist
  partition; verify transient content is discarded and the subset is retained
  across a reboot.
- [x] 3.4 Mount the user partition for user access; verify an update and a
  preserving reinstall leave it unmodified.
- [x] 3.5 Define the directory contract and SSH access; verify a user can modify
  files within the contract and cannot modify the root outside it.
- [x] 3.6 Document the writable contract and directory layout in
  `docs/host-config.md`; verify the documented paths match the running system.

## 4. RAUC A/B updates

- [x] 4.1 Configure RAUC (`efi` bootloader, root slots, keyring); verify
  `rauc status` reports both slots.
- [x] 4.2 Establish signing material and bundle signing; verify a valid bundle
  is accepted and an unsigned or tampered bundle is rejected with no slot
  changes.
- [x] 4.3 Apply a bundle to the inactive slot and its matching UKI; verify only
  the inactive slot is written and the update activates on reboot.
- [x] 4.4 Add USB and signed-HTTPS bundle sources with USB preferred; verify an
  airgapped USB update succeeds and that USB wins when both are available.
- [x] 4.5 Gate updates on an explicit user request; verify no update occurs
  without one.
- [x] 4.6 Expose update status; verify it reports running version, active slot,
  and pending or failed state.
- [x] 4.7a Bump the pinned kernel and the image version, rebuild, and verify the
  new kernel and version are reported after installing the resulting bundle.
- [x] 4.7b Bump the pinned Singularity CE version, rebuild the runtime layer,
  and verify the new version is reported after installing the resulting bundle.
  Requires a second runtime build.
- [x] 4.8 Document the update procedure and signing workflow in
  `docs/os-update.md`; verify the documented commands perform an update.
- [x] 4.9 Add a manual `rollback` option to the update client that reverts to
  the previous slot and restores the prior configuration; verify it activates
  the other slot.

## 5. Health gate and rollback

- [x] 5.1 Snapshot user configuration to the persist partition before applying
  an update; verify the snapshot exists before the inactive slot is written.
- [x] 5.2 Implement the health-gate service to validate over a bounded number of
  boots, restore the prior configuration, and mark the slot good; verify a
  healthy boot marks the slot good.
- [x] 5.3 Wire the health gate to mark the new slot bad and activate the
  previous slot after failed validation; verify a forced validation failure
  reverts to the previous slot and restores configuration.
- [x] 5.4 Make the number of validation boots configurable; verify the
  configured value is honored.
- [x] 5.5 Document the health-gate criteria and rollback behavior in
  `docs/os-update.md`; verify the described behavior matches a forced-failure
  run.

## 7. QEMU verification harness

- [x] 7.1 Build a headless harness that boots a built image under QEMU with OVMF
  UEFI firmware and asserts the system reaches a running state; verify it passes
  for a good image and fails for a deliberately broken one.
- [x] 7.2 Extend the harness to run the installer against a virtual disk, boot
  the installed host, and assert it functions; verify a full install-then-boot
  run passes.
- [x] 7.3a Add functional checks to the harness for the read-only root and the
  writable contract; verify each check fails when its behavior is deliberately
  broken.
- [x] 7.3b Add an update-apply functional check to the harness; verify it fails
  when the update is deliberately broken.
- [x] 7.3c Add an update-revert functional check to the harness; verify it fails
  when revert is deliberately broken. Depends on section 5.
- [ ] 7.4a Verify the harness runs without a display and reports a deterministic
  pass or fail on macOS arm64.
- [x] 7.4b Verify the harness runs without a display and reports a deterministic
  pass or fail on Windows 11 WSL x86_64.
- [x] 7.5 Document how to run the build environment and QEMU harness in
  `verification/README.md`; verify the documented commands reproduce a passing
  run.

## 8. Integration verification

- [x] 8.1 Using the QEMU harness on x86_64, run install -> customize -> update ->
  reboot, then force a failed update; verify headlessly that the system reverts
  and configuration is restored.
- [ ] 8.2 Repeat the full install and update cycle on arm64 hardware; verify it
  behaves as on x86_64.
- [x] 8.3 Verify that a complete install, update, revert, and preserving
  reinstall never modify the user partition.

## 9. Follow-ups

- [ ] 9.1 Build a fully custom realtime kernel from `linux-source` with a
  project config fragment and package it as a Debian package, replacing the
  stock Debian PREEMPT_RT kernel used by task 1.5. Track as a separate change.
