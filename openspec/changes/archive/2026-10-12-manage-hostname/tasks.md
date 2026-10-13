# Tasks

## 1. Build-time hostname seed

- [x] 1.1 Forward `OBKA_HOSTNAME` through `image/build/build.sh` and
  `image/build/build-env/run.sh`, and validate it as an RFC1123 host name,
  failing the build with an error naming `OBKA_HOSTNAME` when invalid. Verify a
  build with a valid name succeeds and an invalid name fails.
- [x] 1.2 Have `image/os/buildroot/board/obake/make-installer.sh` carry the
  seed into the installer service (`Environment=OBKA_HOSTNAME=...`) and verify
  the installer medium reflects the chosen name after install.
- [x] 1.3 Have `image/os/scripts/make-disk.sh` bake
  `persist/etc-overlay/upper/hostname` into `MAKE_DISK` disk images when a
  seed is set, and verify a booted disk reports the seeded name.
- [x] 1.4 Document `OBKA_HOSTNAME` in `image/docs/user-guide.md` and
  `build.sh --help`, and verify the documented invocation runs as written.

## 2. Install-time selection and effective seeding

- [x] 2.1 Re-point `seed_identity` in `image/installer/install.sh` to write the
  identity files (hostname, network configuration, machine-id, SSH host keys)
  into `persist/etc-overlay/upper` via a staging root, and verify a booted
  installed host reflects the seeded `/etc`.
- [x] 2.2 Add hostname resolution to `install.sh` (seed default, interactive
  prompt with the seed as default, RFC1123 validation that re-prompts when
  interactive and fails when not), and verify both modes on a booted host.
- [x] 2.3 Update `image/installer/README.md` and `image/docs/user-guide.md` for
  hostname selection and the overlay seeding location, and verify the
  documented flow sets the chosen name.

## 3. Runtime self-service and /etc/hosts

- [x] 3.1 Add the hostname actions to
  `rootfs-overlay/etc/polkit-1/rules.d/49-obake.rules`
  (`org.freedesktop.hostname1.set-static-hostname`, `set-hostname`) and verify
  `obake` renames the host over SSH without sudo on a booted host.
- [x] 3.2 Remove the `127.0.1.1` hostname line from
  `rootfs-overlay/etc/hosts` and verify the local host name still resolves and
  `localhost` still resolves on a booted host.
- [x] 3.3 Update `image/docs/host-config.md` and `image/docs/user-guide.md` for
  the self-service rename and the `/etc/hosts` change, and verify the
  documented commands run as written.

## 4. Verification harness

- [x] 4.1 Extend `image/verification/qemu/ssh-test.sh` (or `install-test.sh`)
  and the guest checks to assert the seeded host name at first boot, an
  unprivileged rename by `obake`, and persistence of the rename across a slot
  switch; verify the harness passes headlessly.

## 5. Integration

- [x] 5.1 Run the end-to-end flow: build an installer with `OBKA_HOSTNAME`,
  install under QEMU, confirm the seeded name, rename as `obake` without sudo,
  switch slots, and confirm the name persists; verify via the QEMU harness
  output.
- [x] 5.2 Reconcile `image/docs/dev-qemu.md` and any spec cross-references with
  the hostname seeding and rename behavior, and verify the documented dev flow
  works as written.
