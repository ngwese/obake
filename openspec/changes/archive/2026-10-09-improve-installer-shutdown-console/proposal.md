# Proposal

## Why

After a successful interactive install the host powers off immediately, so
operators cannot read the final status on the console. Separately, board
profiles select the installer console (`INSTALLER_CONSOLE`) but the installed
OS UKI always uses a serial console by architecture — on boards like `qdl`
with no serial port, the installed system is undiagnosable from the attached
monitor.

## What Changes

- After install completes (success or failure), the installer waits before
  powering off: interactive builds prompt for confirmation; non-interactive
  builds wait 10 seconds by default.
- **BREAKING**: Rename the build/profile knob `INSTALLER_CONSOLE` to
  `CONSOLE`. Profiles, docs, verification scripts, and the build-env forwarder
  use the new name.
- `CONSOLE` selects the interactive console for both the installer medium and
  the installed OS UKI (and matching getty), so a profile's console choice is
  consistent end-to-end.
- Non-interactive post-install wait duration is set at installer build time
  (default 10 seconds; `0` for CI/automated tooling).

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `system-install`: post-install shutdown wait/confirm; console selection
  renamed from `INSTALLER_CONSOLE` to `CONSOLE` and applied to the installer
  medium.
- `os-image-build`: board profiles supply `CONSOLE` instead of
  `INSTALLER_CONSOLE`; the installed OS UKI (and getty) follow the same
  console selection as the installer.

## Impact

- `image/os/buildroot/board/obake/make-installer.sh` — post-install wait;
  `CONSOLE` wiring.
- `image/os/buildroot/board/obake/post-image.sh` — OS UKI cmdline from
  `CONSOLE`.
- `image/os/buildroot/build.sh` and `image/os/buildroot/profiles/*/*.conf` —
  rename profile key.
- `image/build/build-env/run.sh`, verification QEMU scripts, and docs
  (`image/installer/README.md`, `image/docs/user-guide.md`) — rename and
  document wait behavior.
- No impact on persistent configuration, OTA updates, or shape portability.
- CI/verification must pass a zero wait (e.g. `INSTALLER_POWEROFF_DELAY=0`)
  so automated install tests stay fast.
