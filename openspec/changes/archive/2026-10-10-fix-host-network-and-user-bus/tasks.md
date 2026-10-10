# Tasks

## 1. Optional wired network online

- [x] 1.1 In
      `image/os/buildroot/board/obake/rootfs-overlay/etc/systemd/network/20-obake-wired.network`,
      add `[Link]` / `RequiredForOnline=no` while keeping DHCP match rules.
      Verify: the file contains `RequiredForOnline=no` and still has
      `DHCP=yes` for `en* eth* end*`.
- [x] 1.2 Update the `seed_identity` heredoc in `image/installer/install.sh`
      so the persist-seeded `20-obake-wired.network` matches the overlay.
      Verify: both templates contain the same `RequiredForOnline=no` stanza.
- [x] 1.3 Document optional cabling / wait-online behaviour in
      `image/docs/host-config.md` (and note that existing persist overlays may
      need a one-line edit). Verify: the doc mentions that uncabled boot is
      supported and that OTA does not rewrite an existing seeded `.network`.

## 2. SSH user-bus environment

- [x] 2.1 Add a login-shell hook under the rootfs overlay (e.g.
      `etc/profile.d/obake-user-bus.sh`) that exports `XDG_RUNTIME_DIR` and
      `DBUS_SESSION_BUS_ADDRESS` when `/run/user/$UID` exists and the vars are
      unset. Verify: the script is executable/sourceable and guards on the
      runtime directory.
- [x] 2.2 Document in `image/docs/host-config.md` (or `image/os/README.md`)
      that SSH as `obake` reaches the lingering user bus without PAM.
      Verify: docs mention `systemctl --user` over SSH and the profile.d hook.

## 3. Headless verification

- [x] 3.1 Add a guest check in `image/verification/guest/obake-verify.sh`
      that fails unless the wired `.network` marks interfaces not required for
      online; wire it into the `run` list. Verify: the check function exists
      and is invoked.
- [x] 3.2 Extend `image/verification/qemu/ssh-test.sh` so that after SSH
      login as `obake`, `systemctl --user status` succeeds without the harness
      exporting user-bus env vars. Verify: the new assertion is present and
      does not set `XDG_RUNTIME_DIR` / `DBUS_SESSION_BUS_ADDRESS` in the remote
      command.
- [x] 3.3 Rebuild/reinstall (or OTA) an image that includes the change and run
      the guest suite plus the SSH harness path that exercises the new checks.
      Verify: both new checks pass; on a host with no cable (or after clearing
      a stale persist `.network`), wait-online is not failed for missing
      carrier, and `systemctl --user status` works in a fresh SSH login.
