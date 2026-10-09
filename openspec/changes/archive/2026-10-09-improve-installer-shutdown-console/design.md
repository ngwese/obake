# Design

## Context

See proposal.md for motivation. Today:

- `make-installer.sh` runs `install.sh` then `systemctl poweroff -f` with no
  wait. Interactive vs non-interactive is already encoded as
  `INSTALLER_TARGET_DISK=interactive` vs a fixed device.
- Console for the installer medium comes from `INSTALLER_CONSOLE` (profile or
  env). OS UKI cmdlines in `post-image.sh` ignore that knob and pick a serial
  TTY by architecture (`ttyS0` / `ttyAMA0`, `ttyS2` for `rk35xx`).
- Board profiles under `image/os/buildroot/profiles/<board>/{default,virtual}.conf`
  set `MAKE_INSTALLER`, `INSTALLER_CONSOLE`, and `INSTALLER_TARGET_DISK`.

## Goals / Non-Goals

**Goals:**

- Shared `CONSOLE` knob for installer medium and installed OS UKIs.
- Confirm-before-poweroff for interactive installs; timed wait for
  non-interactive, overridable at build time.
- Keep QEMU/CI paths on serial with zero wait.

**Non-Goals:**

- Generated login passwords (deferred).
- Changing disk-selection UX beyond the post-install wait.
- Runtime (post-boot) console reconfiguration; console is fixed at image build.

## Decisions

1. **Rename `INSTALLER_CONSOLE` → `CONSOLE`**
   - One name for a setting that now owns installer and OS console.
   - Update profiles, `build.sh` allow-list, `build-env/run.sh` forwarder,
     verification scripts, and docs. No compatibility alias for the old name
     (breaking, called out in the proposal).
   - Alternative considered: keep `INSTALLER_CONSOLE` and also read it in
     `post-image.sh` — smaller diff, but the name would be wrong.

2. **Shared console mapping helper logic**
   - Reuse the existing serial/local mapping from `make-installer.sh` in
     `post-image.sh` (same TTY names per arch; `local` → `console=tty0` plus
     serial fallback; `serial` → board serial only).
   - For `rk35xx`, serial remains `ttyS2` (already special-cased).
   - Align getty with the selected console (mask/enable the matching unit, or
     set `BR2_TARGET_GENERIC_GETTY_PORT` / post-build equivalent) so a local
     profile yields a usable VT login, not only kernel messages on `tty0`.
   - Alternative: only change UKI cmdline and rely on systemd generators —
     riskier on boards that already pin getty (rk35xx).

3. **Post-install wait lives in the installer unit wrapper**
   - Keep wait/confirm in the `ExecStart` shell around `install.sh` in
     `make-installer.sh` (or a small helper script it installs), so
     `install.sh` stays focused on partitioning/seeding and still emits
     `OBKA_INSTALL_DONE` / `OBKA_INSTALL_EXIT` before the wait.
   - Interactive when `INSTALLER_TARGET_DISK=interactive` (no `TARGET_DISK`
     in the unit). Non-interactive otherwise.
   - Wait applies on both success and failure so operators/CI can read the
     exit marker.
   - Build-time env `INSTALLER_POWEROFF_DELAY` (seconds, default `10`, `0`
     skips). Baked into the unit/helper at installer build time — not a
     runtime profile file on the target.
   - Alternative: put the wait inside `install.sh` — couples install logic to
     power policy and complicates host-side dry runs.

4. **CI sets delay to zero explicitly**
   - Verification scripts that build installers pass
     `INSTALLER_POWEROFF_DELAY=0`. Default stays 10 for bare-metal
     non-interactive media.
   - Alternative: default 0 when `INSTALLER_TARGET_DISK` is a device path —
     would surprise operators watching a headless serial install.

5. **Artifact naming unchanged**
   - Keep emitting `installer-serial.img` / `installer-local.img` from the
     `CONSOLE` value so existing docs and test expectations stay aligned.

## Risks / Trade-offs

- **[Risk] Interactive prompt blocks forever if no keyboard** → Mitigation:
  only use confirm when the medium was built interactive; bare-metal `qdl`
  default already assumes local keyboard; virtual/CI stay non-interactive.
- **[Risk] CI slows or hangs if delay forgotten** → Mitigation: set
  `INSTALLER_POWEROFF_DELAY=0` in every verification script that builds an
  installer; document in installer README.
- **[Risk] Local OS console without getty still undiagnosable** → Mitigation:
  treat getty alignment as part of the `CONSOLE` work, not optional polish.
- **[Trade-off] Breaking rename** → Accept churn across profiles/docs/scripts
  rather than a misleading `INSTALLER_*` name.

## Migration Plan

- Rename keys in all `profiles/*/*.conf` and call sites in one change.
- Rebuild board images with the intended profile; `qdl`/`default` should show
  local console on both installer and installed UKI cmdlines.
- No OTA or persist-partition migration; only new builds are affected.
- Rollback: revert the change and rebuild; installed hosts keep whatever UKI
  was written at install time until reinstalled or updated.
