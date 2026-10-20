# Proposal

## Why

The image ships `@audio` realtime limits under `/etc/security/limits.d`, but
systemd and OpenSSH are built without PAM, so interactive SSH sessions never
apply them (`ulimit -r` stays 0). Operators debugging on-device, and processes
they start (including Singularity shapes), therefore lack the same realtime
priority and memlock that PipeWire already gets from unit `Limit*` drop-ins.
The no-PAM workarounds (`profile.d` user-bus env, `user@.service.d`
`XDG_RUNTIME_DIR`) paper over session setup; they should be replaced by full
PAM (`pam_systemd` + `pam_limits`).

## What Changes

- **BREAKING** (image rebuild): Enable `linux-pam` so systemd and OpenSSH are
  built with PAM (`+PAM`, `UsePAM yes`).
- Wire SSH and login PAM stacks with `pam_limits` and `pam_systemd` so
  interactive sessions get `@audio` rtprio/memlock and user-runtime env from
  logind.
- Keep `/etc/security/limits.d/obake-audio.conf` as the source of those
  limits (path corrected in docs if needed); ensure it is installed and
  active under PAM.
- Remove no-PAM workarounds:
  - `/etc/profile.d/obake-user-bus.sh`
  - `user@.service.d/20-obake-xdg-runtime.conf`
- Update host docs and verification so SSH login proves both user-bus access
  and realtime rlimits without harness-exported env.
- PipeWire/WirePlumber unit `Limit*` drop-ins stay; they remain the grant for
  linger-started audio services. PAM covers login shells and their children.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `host-config`: SSH login reaches the user bus via PAM/`pam_systemd` instead
  of forbidding PAM and relying on `profile.d`. Image includes full PAM for
  systemd and OpenSSH.
- `host-audio`: Interactive `obake` login sessions SHALL receive the same
  `@audio` realtime priority and memlock limits as documented in
  `limits.d` (via `pam_limits`).
- `image-verification`: SSH checks SHALL assert login-shell realtime limits
  (and keep user-bus reachability without harness env exports).

## Impact

- `image/os/buildroot/configs/obake_base.config` — enable
  `BR2_PACKAGE_LINUX_PAM`.
- Rootfs overlay: remove `profile.d/obake-user-bus.sh` and
  `user@.service.d/20-obake-xdg-runtime.conf`; add/adjust `pam.d` fragments
  if Buildroot defaults omit `pam_limits` / `pam_systemd` on `sshd`.
- `image/os/README.md`, `image/host/SETUP.md` — document PAM + limits; drop
  no-PAM wording.
- `image/verification/qemu/ssh-test.sh` (and related) — assert `ulimit -r`
  / memlock after SSH login.
- OTA: next image rebuild replaces the rootfs; no user-home migration.
  Persist overlays that copied old drop-ins are unlikely (those paths are
  read-only image files).
- Shape portability unchanged; shapes launched from a PAM login inherit
  session rlimits. Future `obake-sessiond` as a user unit should still set
  its own `Limit*` (out of scope here).
