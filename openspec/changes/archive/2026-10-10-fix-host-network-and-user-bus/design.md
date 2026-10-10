# Design

## Context

See proposal.md for motivation. Today:

- `20-obake-wired.network` (image overlay and installer persist seed) matches
  `en* eth* end*` with `DHCP=yes` and no `RequiredForOnline=`. Default
  wait-online therefore waits for those links and fails when none have
  carrier. The unit is enabled under `multi-user.target.wants/`.
- `obake-linger.service` enables linger and starts `user@<uid>`;
  `/var/lib/systemd` is persisted. `user@.service.d` sets `XDG_RUNTIME_DIR`
  for the manager because systemd is built without PAM. There is no
  login-shell hook, so SSH sessions lack the locator env vars even when
  `/run/user/<uid>/bus` exists.

Guest `obake-verify.sh` runs as `User=obake` via a system unit (not a login
shell), so user-bus env must be checked in `ssh-test.sh`. Optional cabling can
be asserted from the active `.network` file in the guest suite (QEMU usually
has a virtual NIC, so "wait-online failed" is not a reliable negative signal).

## Goals / Non-Goals

**Goals:**
- Uncabled boot does not fail wait-online for wired links
- DHCP unchanged when a cable is present
- SSH login shells can use `systemctl --user` / session D-Bus without PAM
- Headless checks cover both behaviours

**Non-Goals:**
- Enabling `linux-pam` / `pam_systemd`
- Disabling `systemd-networkd` or DHCP
- Auto-migrating already-written persist overlay copies of
  `20-obake-wired.network` on OTA
- Changing linger / `obake-linger.service` (already correct)
- Fixing media automount (separate change)

## Decisions

### 1. `RequiredForOnline=no` on the wired `.network`

Add under `[Link]` (or the section systemd documents for this knob):

```
[Link]
RequiredForOnline=no
```

in both:

- `rootfs-overlay/etc/systemd/network/20-obake-wired.network`
- installer `seed_identity` heredoc in `image/installer/install.sh`

Rationale: matches the "cabling is optional" policy; wait-online succeeds with
zero required links; DHCP behaviour on carrier is unchanged. Alternatives
considered: mask wait-online (also works, but drops `network-online.target`
semantics entirely); `--any` (still fails with zero cables).

### 2. Login-shell env via `/etc/profile.d`

Ship something like `etc/profile.d/obake-user-bus.sh` that, for a normal
user with `/run/user/$UID` present, exports:

```
XDG_RUNTIME_DIR=/run/user/$UID
DBUS_SESSION_BUS_ADDRESS=unix:path=$XDG_RUNTIME_DIR/bus
```

only when unset (do not clobber an explicit environment). Guard on directory
existence so non-lingering accounts are unaffected.

Rationale: image already rejected PAM for size/complexity; profile.d covers
interactive SSH login shells (bash login). Alternatives considered: enable PAM
+ pam_systemd (heavier); document `systemctl --user -M obake@` (poor UX).

### 3. Verification split: guest file check + SSH harness

- Guest (`obake-verify.sh`): assert the wired `.network` content includes
  `RequiredForOnline=no` (read `/etc/systemd/network/20-obake-wired.network`
  or equivalent active path).
- SSH (`ssh-test.sh`): after login, run `systemctl --user status` with no
  prior env exports in the remote command.

Rationale: guest unit is not a login shell; SSH harness already proves the
directory contract and diagnostics over a real SSH session.

## Risks / Trade-offs

- **[Persist overlay keeps old `.network`]** → Existing hosts that already
  seeded `20-obake-wired.network` into the `/etc` overlay will not pick up
  `RequiredForOnline=no` from OTA alone. Mitigation: document a one-line
  overlay edit / reinstall seed; new installs get the fixed template.
- **[Non-login SSH remote commands]** → `ssh host cmd` without a login shell
  may skip `profile.d`. Mitigation: `ssh-test.sh` and normal operator SSH
  use login shells; document `bash -l` if needed for non-interactive scripts.
- **[network-online becomes a no-op for wired]** → Acceptable: no production
  host units currently depend on waiting for carrier; optional cabling is the
  product contract.

## Migration Plan

1. Ship image + installer template changes.
2. New installs: correct seed on first boot.
3. Existing persist: operator adds `RequiredForOnline=no` under `[Link]` in
   `/etc/systemd/network/20-obake-wired.network` (persist overlay), or
   replaces that file; reboot or `networkctl reload` as appropriate.
4. Rollback: revert the two files and the profile.d hook; no schema change.
