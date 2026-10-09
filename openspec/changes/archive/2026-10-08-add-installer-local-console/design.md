# Design

## Context

See `proposal.md` for motivation.

The `qdl` kernel (`board/obake/qdl/linux.config`) already enables virtual
terminals (`CONFIG_VT`, `CONFIG_VT_CONSOLE`) and a USB input stack (`USB_XHCI`,
`HID`, `USB_HID`, `INPUT_EVDEV`), but it has no console that can draw to a
screen: `CONFIG_FB` and `CONFIG_DRM` are unset, so `CONFIG_FRAMEBUFFER_CONSOLE`
is not even offered, and the legacy `CONFIG_VGA_CONSOLE` cannot bind UEFI's GOP
framebuffer. The pinned 7.2.9 kernel provides `DRM_SIMPLEDRM` and
`SYSFB_SIMPLEFB` under `drivers/gpu/drm/sysfb` for exactly this case.

The installer medium is assembled by
`board/obake/make-installer.sh` (from `post-image.sh` when `MAKE_INSTALLER=1`).
It hardcodes the interactive console to `ttyS0` for x86_64: the generated
`obake-installer.service` uses `TTYPath=/dev/ttyS0` with stdin redirected from
that device, masks `serial-getty@ttyS0`, and the installer UKI command line is
`console=ttyS0`. The installed host's UKI is likewise `console=$cons`
(`post-image.sh`). The QEMU installer harnesses drive that serial console.

## Goals / Non-Goals

**Goals:**

- Give the `qdl` image a working local text console on the UEFI GOP framebuffer
  without a GPU driver excluded by PREEMPT_RT.
- Let the installer medium run on a local virtual terminal, driven by an
  attached USB keyboard, selected at build time.
- Preserve the serial console as the default so headless CI and the QEMU
  harnesses are unchanged.

**Non-Goals:**

- Changing `installer/install.sh` logic, the partition layout, or the install
  flow.
- A local console for the installed host, or a graphical interface at any
  point; the host remains headless and reached over the network.
- A local-console path for `aarch64` boards (no monitor/keyboard target today).
- A graphical QEMU verification harness; console rendering is smoke-tested on
  hardware.

## Decisions

1. **Firmware framebuffer via DRM simpledrm** — add `CONFIG_DRM=y`,
   `CONFIG_DRM_SIMPLEDRM=y`, `CONFIG_SYSFB=y`, `CONFIG_SYSFB_SIMPLEFB=y`,
   `CONFIG_DRM_FBDEV_EMULATION=y`, and `CONFIG_FRAMEBUFFER_CONSOLE=y` to
   `qdl/linux.config`. `DRM_FBDEV_EMULATION` selects `FB_CORE`, which
   `FRAMEBUFFER_CONSOLE` requires. Alternatives: classic efifb
   (`CONFIG_FB`, `CONFIG_FB_EFI`, `CONFIG_FRAMEBUFFER_CONSOLE`), or the new
   7.2 `CONFIG_DRM_EFIDRM` driver (which conflicts with `SYSFB_SIMPLEFB`).
   SimplEdrm is the path the `add-qdl-board` design already anticipated and
   handles both EFI and VESA firmware framebuffers. `i915` stays off because it
   depends on `!PREEMPT_RT`.

2. **Console selection knob, serial default** — `INSTALLER_CONSOLE=serial|local`
   read by `make-installer.sh`, defaulting to `serial`. `serial` reproduces
   today's `ttyS0`/`ttyAMA0` behavior, so verification and existing workflows do
   not change. An unrecognized value fails the build. Alternative: a
   board-derived default; rejected because the same board boots under QEMU
   (serial) and on metal (local), so the operator must choose.

   The selection is a post-image property: switching it re-runs only
   `make-installer.sh` (the kernel, packages, and `TARGET_DIR` are reused), so
   both a serial and a local installer medium are produced from one base build
   with no full rebuild. Both media are byte-identical apart from the installer
   UKI command line and the installer service unit, which the automated checks
   assert. Alternative: runtime dual-console selection so one artifact serves
   both; rejected as unnecessary complexity here, since the serial medium gates
   the shared install logic under QEMU and the local differences are narrow,
   asserted, and hardware-smoke-tested.

   Each build names its medium for the console it carries
   (`installer-serial.img`, `installer-local.img`). The QEMU installer
   harnesses look for `installer-serial.img` and build the serial medium when
   it is absent, so a local-only build is never mistaken for a serial one and a
   missing serial medium is handled gracefully.

3. **Local console on `tty1`, not `tty0`** — with `local`, the service uses
   `TTYPath=/dev/tty1` and stdin from `/dev/tty1`. systemd's `TTYPath` needs a
   concrete VT, so `tty0` (the active-VT alias) cannot be used. The UKI command
   line adds `console=tty0` so the VT receives kernel and fbcon output, keeping
   the serial console too as a fallback. The service masks `getty@tty1.service`
   (instead of `serial-getty@<serial>`) so a login prompt cannot race the
   installer for the VT.

4. **Scope the local console to the installer medium** — `post-image.sh`'s host
   UKI command line is left unchanged. The installed host is headless by
   design and managed over the network; adding local output there is a
   separate, follow-up decision.

5. **Verification by config and command-line assertions plus hardware smoke** —
   automated checks assert the built kernel enables the framebuffer-console
   symbols and that a `local` installer's UKI command line and service target
   `tty0`/`tty1`; the existing serial QEMU harnesses remain the regression
   gate. The harnesses run `generic-x86_64` because the `qdl` kernel has no
   virtio drivers and cannot boot under QEMU; the qdl medium is covered by the
   config/command-line assertions and the hardware smoke test. Rendering,
   firmware GOP, and keyboard behavior are smoke-tested on the Quieter DL,
   matching how IGC/MIDI 2.0 hardware is handled. Alternative: a QEMU
   `screendump` screenshot check; rejected as brittle and firmware-dependent.

## Risks / Trade-offs

- **[Risk] Firmware may not expose a GOP framebuffer at the expected
  resolution** → Mitigation: simpledrm falls back to no console, so keep the
  serial console in the local command line; smoke-test on the target.
- **[Risk] Local console selection could disturb the headless CI path** →
  Mitigation: `serial` stays the default and the QEMU installer harness runs it
  unchanged.
- **[Risk] Enabling `CONFIG_DRM` could pull in an RT-incompatible driver** →
  Mitigation: assert the built `qdl` `.config` still has `CONFIG_DRM_I915`
  unset and `CONFIG_PREEMPT_RT=y`.
- **[Risk] `getty@tty1` races the installer for the VT** → Mitigation: mask
  the matching getty unit in `local` mode.

## Migration Plan

- Additive and board-scoped: existing serial builds and harnesses are
  unaffected.
- From one base build, produce a serial medium for QEMU verification and a
  local medium for hardware with `MAKE_INSTALLER=1 INSTALLER_CONSOLE=<value>`;
  each only re-runs the post-image hook. Roll back by omitting
  `INSTALLER_CONSOLE` (serial).
- No pin, partition-layout, OTA, or shape migration.
