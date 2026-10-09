# Design

## Context

See proposal.md for motivation. Boards today are `generic-aarch64`,
`generic-x86_64`, and `rk35xx`, each formed as `obake_base.config` +
`obake-<board>.config`. Kernels use a board base config plus shared
`linux-realtime.fragment` and `linux-obake.fragment`. The pinned kernel is
already 7.2.9. Curieux's `quieter_dl_aoip_defconfig` is a fat 6.12 guide for
Quieter DL hardware; it is not imported wholesale and has MIDI 2.0 disabled.
The git-lfs vendored download cache was populated from an aarch64 build and
predates the shared-base disk tooling and the x86_64 systemd-boot path; the qdl
build completes it.

## Goals / Non-Goals

**Goals:**

- Add `qdl` with the same board split and EFI/systemd-boot path as
  `generic-x86_64`.
- Ship a short curated `linux.config` for N100 + I226 + NVMe + HDA/USB MIDI 2.0
  + AOIP plumbing on 7.2.9.
- Keep `NO_HZ_IDLE` via the shared realtime fragment.

**Non-Goals:**

- AES67/Ravenna kmod or userspace, isolcpus, `NO_HZ_FULL`, IRQ affinity.
- Wireless drivers or firmware.
- Kernel pin bumps or changes to RAUC/partition layout.
- QEMU substitute for hardware smoke (I226 / USB MIDI 2.0).

## Decisions

1. **Board id `qdl`** — short, matches product nickname; overlay sets
   `board=qdl` / `arch=x86_64`. Alternative: `quieter-dl` (longer, less
   convenient as a build argument).

2. **Minimal custom config, not quieter paste** — follow
   `generic-x86_64/linux.config` style; use quieter only as a checklist for
   IGC, NVMe, HDA, PTP/XDP. Alternative: arch defconfig + fragments (too
   large); or import quieter (stale, non-minimal, MIDI 2.0 off).

3. **MIDI 2.0 via USB audio** — enable `CONFIG_SND_USB_AUDIO_MIDI_V2` and
   `CONFIG_SND_UMP_LEGACY_RAWMIDI` so UMP and legacy `/dev/snd/midi*` exist.
   Sequencer UMP follows Kconfig selects.

4. **AOIP plumbing only** — `CONFIG_IGC`, `CONFIG_PTP_1588_CLOCK`,
   `CONFIG_NETWORK_PHY_TIMESTAMPING`, `CONFIG_XDP_SOCKETS`. Defer tuning
   profiles and AOIP stacks.

5. **No wireless** — leave Wi-Fi/Bluetooth unset to shrink surface and avoid
   firmware blobs.

6. **No i915 (PREEMPT_RT)** — `CONFIG_DRM_I915` depends on `!PREEMPT_RT`, so
   the realtime fragment rules it out. Use HDA (Realtek + HDMI codec) for
   audio; optional `DRM_SIMPLEDRM`/`SYSFB_SIMPLEFB` only if a firmware FB is
   needed later. HDMI/DP display bring-up on RT is out of scope for this
   change.

7. **Realtime owned by fragment** — board config must not set `PREEMPT_RT`,
   `HZ_*`, or `NO_HZ_*`; shared fragment supplies `NO_HZ_IDLE`.

8. **Complete the vendored cache** — `vendor/dl/` is the committed offline
   cache. Add the sources the shared base and x86_64 EFI path need but the
   aarch64-derived snapshot predates (`gptfdisk`, `dosfstools`, `parted`,
   `lvm2`, `go` linux-amd64, `gnu-efi`, `python-pyelftools`) so a qdl build
   resolves every dependency without network access. Alternative: fetch at
   build time, breaking the pinned-input contract.

## Risks / Trade-offs

- **[Risk] Missing SoC glue for N100 boot** → Mitigation: enable ACPI,
  `X86_INTEL_LPSS`, Alder Lake pinctrl, NVMe, XHCI; iterate from hardware
  dmesg if a dependency is omitted.
- **[Risk] 7.2.9 Kconfig renames vs quieter 6.12** → Mitigation: build once,
  fix with `olddefconfig` / explicit symbols; do not paste quieter.
- **[Risk] Go VCS stamping breaks Singularity under Buildroot `GIT_DIR=.`**
  → Mitigation: package patch
  `0002-go-disable-vcs-stamping.patch` adds `-buildvcs=false` to
  Singularity's `GOFLAGS` (shared package, not board-specific).
- **[Risk] aarch64-derived vendor snapshot lacks shared/x86_64 sources**
  → Mitigation: commit the sources the qdl build fetched (`gptfdisk`,
  `dosfstools`, `parted`, `lvm2`, `go` linux-amd64, `gnu-efi`,
  `python-pyelftools`) so offline builds stay complete.
- **[Trade-off] No QEMU coverage for IGC/MIDI2** → Accept hardware smoke for
  those; `check-boards.sh` + full image build still gate CI-style checks.
- **[Trade-off] No i915 under PREEMPT_RT** → HDA (Realtek + HDMI codec) only;
  display GPU bring-up deferred.

## Migration Plan

- Additive board only; existing boards unchanged.
- Operators build with `image/os/buildroot/build.sh qdl` and install like
  other x86_64 EFI images.
- Rollback: stop building/shipping `qdl`; no data migration.
