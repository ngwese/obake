# Proposal

## Why

The `system-install` spec promises an installer that runs over a serial console
**or** a text console with an attached keyboard and monitor, but the `qdl`
(MeLE Quieter DL) image cannot deliver the text-console path. Its kernel is
built with virtual terminals but no framebuffer console, so UEFI's graphics
framebuffer is never drawn, and the installer service is hardwired to `ttyS0`.
An operator with an HDMI monitor and USB keyboard — and no serial port on this
board — cannot install obake.

## What Changes

- Add an EFI framebuffer text console to the `qdl` kernel: a
  firmware-framebuffer DRM driver with fbdev emulation and the framebuffer
  console, so UEFI's GOP framebuffer drives a virtual terminal. This does not
  depend on `i915`, which the realtime fragment excludes.
- Add a build-time installer console selection, `INSTALLER_CONSOLE=serial|local`
  (default `serial`). `local` runs the installer service on `tty1` and adds
  `console=tty0` to the installer UKI command line; `serial` preserves today's
  behavior so headless CI is unchanged.
- Document the local-console build and bare-metal flow in
  `image/installer/README.md`, and forward the knob through the verification
  build environment.
- Leave the installed host's console and command line unchanged; the selection
  applies to the installer medium only.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `system-install`: the installer console is selectable at build time and the
  local text-console path is realizable on hardware without a serial port.
- `os-image-build`: the `qdl` board kernel includes an EFI framebuffer console
  for local text output.

## Impact

- `image/os/buildroot/board/obake/qdl/linux.config` — framebuffer console
  options (DRM simpledrm / sysfb / fbdev emulation / framebuffer console).
- `image/os/buildroot/board/obake/make-installer.sh` — console selection;
  installer service tty and UKI command line.
- `image/verification/build-env/run.sh` — forward `INSTALLER_CONSOLE` so the
  containerized build path can select it.
- `image/installer/README.md` — document the knob and the bare-metal flow.
- Verification: keep the serial default and its QEMU harnesses green; assert
  the local-console kernel config and installer command line; hardware smoke
  test on the Quieter DL.
- No pin bumps, no partition-layout, OTA, or shape changes.
