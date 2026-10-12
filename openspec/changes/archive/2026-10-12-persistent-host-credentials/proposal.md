# Proposal

## Why

Build-time credentials are meant to be seeds, but today they behave as
fragile, permanently baked state. `OBKA_SSH_AUTHORIZED_KEY` is documented but
silently ignored whenever the key file lives outside the build tree, because
the build container only mounts the repository, and it fails outright on macOS
where `realpath -m` is unsupported. When a key does get baked, it lands in
root-owned `/etc/ssh/authorized_keys.d/`, so the `obake` user can neither
replace it nor stop relying on it. There is no supported way to supply a key
alongside the installer, and no way for the `obake` user to change its own
password. The result is that consumers of a prebuilt image fall back to baking
a default password, logging in, copying a key by hand, and then changing the
password.

## What Changes

- Fix `OBKA_SSH_AUTHORIZED_KEY` handling so a key file is honored regardless
  of its location on the build host (inside or outside the build tree) and on
  both macOS and Linux build hosts; a path that names no readable file fails
  the build instead of being silently skipped.
- Treat build-time credentials as seeds: the key is embedded in the installer
  medium and provisioned once at install; the password remains a rootfs seed.
- Seed the `obake` authorized key into `~/.ssh/authorized_keys` on the user
  partition at install, only when the user has no key, so a reinstall that
  preserves user data never overwrites a user-managed key.
- Let the `obake` user replace its authorized key and change its own password
  without elevated privileges; both changes persist across boots, slot
  switches, and OS updates.
- Stop seeding a permanently honored key under `/etc/ssh/authorized_keys.d/`,
  so replacing the key actually retires the seed.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `os-image-build`: add a build-time SSH authorized key seed requirement that
  is host-path and host-platform independent; restate the build-time password
  as a seed that is replaced on update only until the user changes it.
- `system-install`: add installer seeding of the `obake` authorized key into
  the user partition, applied only when absent.
- `host-config`: add self-service management of the `obake` authorized key and
  password, read from `~/.ssh/authorized_keys`, with changes surviving updates.
- `image-verification`: add after-boot checks for credential seeding, self
  service, and persistence across a slot switch.

## Impact

- Build/install: `image/build/build.sh`, `image/build/build-env/run.sh`,
  `image/os/buildroot/board/obake/post-build.sh`,
  `post-fakeroot.sh`, `make-installer.sh`, `image/installer/install.sh`.
- Runtime config: `rootfs-overlay/etc/ssh/sshd_config.d/obake.conf`,
  `rootfs-overlay/etc/tmpfiles.d/obake-home.conf`, and the image's busybox
  `passwd` (setuid) for password self-service.
- Docs: `image/docs/user-guide.md`, `image/docs/host-config.md`,
  `image/docs/dev-qemu.md`, `image/installer/README.md`.
- Verification: `image/verification/qemu/ssh-test.sh`,
  `password-test.sh`, `persist-test.sh`, and guest checks.
- Persistent configuration and OTA: user-managed credentials live on the user
  partition and the `/etc` overlay, which are retained across updates and slot
  switches; an unchanged password seed continues to be replaced by an update,
  while a user-changed password is not.
- Cross-host portability: build hosts on macOS and Linux behave the same for
  credential inputs.
