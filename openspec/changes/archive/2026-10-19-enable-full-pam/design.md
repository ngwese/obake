# Design

## Context

See proposal.md for motivation. Current constraints that shape the approach:

- `obake_base.config` enables systemd (logind, networkd, resolved) and
  OpenSSH, but not `BR2_PACKAGE_LINUX_PAM`. Buildroot then builds systemd
  `--disable-pam` and OpenSSH `--without-pam`.
- Overlay already ships
  `etc/security/limits.d/obake-audio.conf` (`@audio` rtprio 95, memlock
  unlimited). `obake` is in group `audio` via `users.txt`. Without
  `pam_limits`, login shells ignore that file (`ulimit -r` = 0).
- No-PAM workarounds in the overlay:
  - `etc/profile.d/obake-user-bus.sh` — exports `XDG_RUNTIME_DIR` /
    `DBUS_SESSION_BUS_ADDRESS` for login shells
  - `usr/lib/systemd/system/user@.service.d/20-obake-xdg-runtime.conf` —
    forces `XDG_RUNTIME_DIR` so `user@` can start without `pam_systemd`
- `obake-linger.service` already runs `loginctl enable-linger`,
  `user-runtime-dir@`, and `user@` at boot.
- PipeWire/WirePlumber keep unit `LimitRTPRIO` / `LimitMEMLOCK` drop-ins;
  those stay as the linger-path grant.
- `ssh-test.sh` already checks `systemctl --user` after SSH without harness
  env exports; it does not yet check rlimits.

## Goals / Non-Goals

**Goals:**

- Full PAM on the image: linux-pam, systemd `+PAM`, OpenSSH `UsePAM yes`.
- SSH/login sessions apply `@audio` limits and get user-runtime env from
  `pam_systemd` / `pam_limits`.
- Delete the two no-PAM workarounds above; docs and specs match.

**Non-Goals:**

- `obake-sessiond` or changing how shapes are launched in production.
- Silencing `wpctl` / client `module-rt` via `client.conf` (separate concern).
- Packaging rtkit or changing PipeWire/WirePlumber versions.
- Replacing unit `Limit*` drop-ins with PAM-only grants for audio services.
- Touching the user `dbus.socket` `set-environment` helper (not a login
  `profile.d` workaround; still useful inside the user manager).

## Decisions

### Enable `BR2_PACKAGE_LINUX_PAM` in the shared base config

Add `BR2_PACKAGE_LINUX_PAM=y` to `obake_base.config`. Buildroot then:

- Builds systemd with PAM (`pam_systemd`, `PAMName=` on `user@`).
- Builds OpenSSH `--with-pam`, installs `/etc/pam.d/sshd`, and sets
  `UsePAM yes` in `sshd_config`.

One knob covers the stack the proposal calls "full PAM". Alternative:
hand-patched systemd/OpenSSH recipes — rejected; use the distro switch.

### Ensure `sshd` (and login) session stacks include limits + systemd

After Buildroot install, verify `/etc/pam.d/sshd` session lines include:

- `pam_limits.so` (applies `limits.d`)
- `pam_systemd.so` (runtime dir / session bus env for the login)

If the stock `sshd.pam.generic` omits either, ship an overlay fragment or a
small `post-build.sh` sed/install that adds them. Prefer the smallest
overlay under `rootfs-overlay/etc/pam.d/` when a full file replace is
clearer than sed. Same check for `/etc/pam.d/login` if console login is
in scope (headless SSH is the primary path; keep login consistent).

Keep `etc/security/limits.d/obake-audio.conf` unchanged unless a path or
comment fix is needed.

### Remove no-PAM workarounds once PAM is on

Delete:

- `etc/profile.d/obake-user-bus.sh`
- `usr/lib/systemd/system/user@.service.d/20-obake-xdg-runtime.conf`

Rationale: with PAM, `user@.service` uses the systemd-user PAM stack /
`user-runtime-dir@` (already started by linger) for the runtime directory,
and SSH sessions get env from `pam_systemd`. Leaving the drop-in would mask
regressions if PAM session setup fails.

Alternative considered: keep the `user@.service.d` env as a belt-and-suspenders
fallback. Rejected; the change's point is to rely on PAM and delete the
bypass.

### Verification extends `ssh-test.sh`

After the existing `systemctl --user status` check, assert in the same
login shell:

- `ulimit -r` equals `95`
- `ulimit -l` is unlimited (accept `unlimited` / `infinity` spelling)

Do not export rlimits in the harness. Guest `obake-verify.sh` stays on unit
`LimitRTPRIO` checks for PipeWire/WirePlumber; login limits are an SSH-path
concern.

### Docs

Update `image/os/README.md` (and `SETUP.md` if it still describes no-PAM
user-bus setup) to state PAM + `limits.d` for login shells, and that unit
drop-ins remain for PipeWire/WirePlumber.

## Risks / Trade-offs

- [Buildroot `sshd` PAM file lacks `pam_systemd`] → Login env regresses;
  `systemctl --user` fails in `ssh-test.sh`. Mitigate by overlay/post-build
  assert and explicit session lines in tasks.
- [Removing `user@.service.d` breaks linger `user@` start] → Boot has no
  PipeWire. Mitigate: QEMU guest checks still require active pipewire /
  wireplumber; fix PAMName / `user-runtime-dir@` before merge.
- [Image size / attack surface] → linux-pam is small relative to the audio
  stack; accepted in exchange for correct login rlimits.
- [Persist or custom `sshd_config` disables UsePAM] → Unlikely (image owns
  sshd_config); drop-in directory still included via post-build Include.
- [Children of non-login paths still have rtprio 0] → Expected; production
  sessiond should use unit `Limit*` (out of scope).

## Migration Plan

Ships with the next OS image / OTA. No home-directory migration. Rollback:
boot previous slot. Operators who copied `obake-user-bus.sh` into a writable
overlay should remove it after upgrade (rare; profile.d is on the image).

## Open Questions

None.
