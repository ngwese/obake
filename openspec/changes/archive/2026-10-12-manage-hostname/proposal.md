# Proposal

## Why

The host name is fixed at `obake`. There is no first-class way for the
`obake` user to rename the host after install — unprivileged
`hostnamectl set-hostname` is denied by polkit, and the documented
install-time seed (`OBKA_HOSTNAME`) is never forwarded and is written to a
persist path the runtime `/etc` overlay does not use, so it has no effect.
Hosts therefore cannot be given meaningful names at provision time or later.

## What Changes

- Let the `obake` user rename the host over SSH without elevated privileges by
  adding a polkit grant for the systemd hostname actions; the name persists
  across boots, slot switches, and updates.
- Add a build-time `OBKA_HOSTNAME` seed, forwarded through the build like the
  SSH key seed, that the installer medium and `MAKE_DISK` images carry.
- At install, seed the identity files into the persist `/etc` overlay that the
  runtime actually uses (fixing hostname, network, machine-id, and SSH host
  key seeding), and support an interactive hostname prompt with the seed as the
  default plus validation as an RFC1123 name.
- Bake the hostname into the persist overlay of `MAKE_DISK` images so a flashed
  disk boots with the chosen name.
- Stop carrying the hostname in `/etc/hosts`; local resolution already comes
  from `systemd-resolved` and `myhostname`.
- Document the identity-seeding location, the self-service rename, and the
  `/etc/hosts` change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `os-image-build`: add a build-time hostname seed that is forwarded into the
  installer medium and `MAKE_DISK` images.
- `system-install`: make machine-specific seeding land on the persist `/etc`
  overlay the runtime uses, and add hostname selection (interactive prompt or
  non-interactive seed).
- `host-config`: add a self-service host rename without elevated privileges and
  reflect that `/etc/hosts` no longer carries the hostname.
- `image-verification`: add checks for hostname seeding at install, the
  unprivileged rename, and persistence across a slot switch.

## Impact

- Build/install: `image/build/build.sh`, `image/build/build-env/run.sh`,
  `image/os/buildroot/board/obake/make-installer.sh`,
  `image/os/scripts/make-disk.sh`, `image/installer/install.sh`.
- Runtime config: `rootfs-overlay/etc/polkit-1/rules.d/49-obake.rules` and
  `rootfs-overlay/etc/hosts`.
- Docs: `image/docs/host-config.md`, `image/docs/user-guide.md`,
  `image/docs/dev-qemu.md`, `image/installer/README.md`.
- Verification: `image/verification/qemu/install-test.sh`,
  `image/verification/qemu/ssh-test.sh`, guest checks.
- Persistent configuration and OTA: the hostname and the other identity seeds
  live in the persist `/etc` overlay, which updates and slot switches retain;
  a rename through `hostnamectl` is retained the same way.
