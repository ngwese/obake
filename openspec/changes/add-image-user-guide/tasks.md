# Tasks

## 1. Prerequisite

- [ ] 1.1 Confirm `add-installer-local-console` has landed before writing the
  bare-metal section: `INSTALLER_CONSOLE` and the `local`/`tty1` path are
  present in `image/os/buildroot/board/obake/make-installer.sh` and documented
  in `image/installer/README.md`. Verify with
  `grep -n INSTALLER_CONSOLE image/os/buildroot/board/obake/make-installer.sh`.

## 2. Guide skeleton and orientation

- [ ] 2.1 Create `image/docs/user-guide.md` with an introduction, the
  repository/artifact map, and the build -> install -> update through-line
  that links to the sibling docs instead of restating them. Verify the file
  exists and lists every planned section heading.
- [ ] 2.2 Link the guide from the docs row of `image/README.md` and, where it
  aids navigation, from `image/docs/{host-config,os-update}.md`. Verify each
  added relative link resolves to an existing file.

## 3. Build and QEMU chapters

- [ ] 3.1 Write the "build system structure" and "building an image" sections:
  shared base plus per-board fragments, `pins.json`, runtime meta-packages,
  emitted artifacts, the containerized `image/verification/build.sh` path, and
  `MAKE_INSTALLER=1`. Verify the documented boards, commands, and paths match
  `image/os/README.md` and `image/verification/build.sh`.
- [ ] 3.2 Write the "what QEMU verifies and what only hardware can" section,
  stating both sides of the boundary explicitly. Verify every QEMU claim maps
  to a harness listed by `ls image/verification/qemu`, and every
  hardware-only claim names the smoke test it depends on.

## 4. Bare-metal install (x86_64 / qdl)

- [ ] 4.1 Write the pre-install gathering section: target board, architecture,
  destination storage device name, and required console. Verify the storage
  example matches the installer's default in
  `image/os/buildroot/board/obake/make-installer.sh` and the
  `TARGET_DISK` override in `image/installer/install.sh`.
- [ ] 4.2 Write the USB installer write, boot, and target-disk selection
  section using the local console and the interactive disk prompt, and state
  that `rk35xx`/arm64 is out of scope for this section. Verify the documented
  knob names and the interactive flow match `make-installer.sh` and
  `install.sh`.

## 5. SSH, filesystem layout, and USB update

- [ ] 5.1 Write the SSH access section: generating a key with `ssh-keygen`,
  seeding the public half via `OBKA_SSH_AUTHORIZED_KEY` at build or install
  time, and where the private half is kept. Verify the variable names match
  `image/os/buildroot/board/obake/post-build.sh` and
  `image/installer/install.sh`, and that the section states no SSH keypair is
  generated at build time.
- [ ] 5.2 Write the filesystem layout section (read-only root vs the writable
  contract), linking `image/docs/host-config.md` for detail. Verify the paths
  and persistence rules match the table in `host-config.md`.
- [ ] 5.3 Write the USB update section: signing a bundle, placing the
  `*.raucb` on any partition, and `obake-update status|apply|rollback`,
  linking `image/docs/os-update.md`. Verify the discovery rule stated matches
  the `find -maxdepth 2 -name '*.raucb'` logic in
  `image/os/buildroot/board/obake/rootfs-overlay/usr/bin/obake-update`.

## 6. Integration verification

- [ ] 6.1 Validate the change and its links headlessly:
  `openspec validate add-image-user-guide --strict` passes, and a scripted
  check confirms every file/script path referenced by the guide exists.
- [ ] 6.2 Confirm no pinned version changed, so no pin bump is required:
  `git -C image status --porcelain` shows no changes under
  `image/os/buildroot/pins.json` or `image/os/buildroot/VERSION`.
