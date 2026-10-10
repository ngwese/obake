# Proposal

## Why

Uncabled NICs are a normal operating mode for this headless host, but
`systemd-networkd-wait-online` still requires managed links to come online, so
boot always lands in a degraded state when no cable is present. Separately, the
`obake` user manager already runs at boot via linger, yet SSH login shells do
not receive `XDG_RUNTIME_DIR` / `DBUS_SESSION_BUS_ADDRESS` (systemd is built
without PAM), so `systemctl --user` fails even though `/run/user/<uid>` exists.

## What Changes

- Mark wired networkd configuration so managed interfaces are not required for
  `network-online` (uncabled boot no longer fails wait-online).
- Keep the same network seed in the installer persist template so new installs
  match the image.
- Inject user-bus locator environment variables into login shells when the
  linger runtime directory is present, without enabling PAM.
- Extend headless verification (guest and/or SSH harness) so these behaviours
  regress visibly.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `host-config`: require that network online is not a boot requirement for
  wired interfaces, and that an SSH login as `obake` can use `systemctl --user`
  against the lingering user manager without manually exporting environment
  variables.
- `image-verification`: guest/SSH checks must fail when wait-online is left
  requiring cabling, or when an SSH login as `obake` cannot talk to the user
  bus.

## Impact

- `image/os/buildroot/board/obake/rootfs-overlay/etc/systemd/network/20-obake-wired.network`
- `image/installer/install.sh` (`seed_identity` network template)
- New login-shell env hook under the rootfs overlay (e.g. `etc/profile.d/`)
- `image/docs/host-config.md` network / user-session notes
- `image/verification/guest/obake-verify.sh` and/or
  `image/verification/qemu/ssh-test.sh`
- Persist: hosts that already have an overlay-copied
  `20-obake-wired.network` keep the old file until that overlay entry is
  updated or recreated; OTA alone does not rewrite persist-seeded network
  config
