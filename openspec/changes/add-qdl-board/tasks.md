# Tasks

## 1. Board wiring

- [x] 1.1 Add `image/os/buildroot/configs/obake-qdl.config` mirroring
      `obake-generic-x86_64.config` (x86_64, systemd-boot, dual overlay,
      custom kernel path) and verify `check-boards.sh` reports
      `OBKA_BOARD_BASE_PASS` with no base-symbol overlap for `qdl`
- [x] 1.2 Add `board/obake/qdl/rootfs-overlay/etc/obake/board` (`board=qdl`,
      `arch=x86_64`) and `tuning-profile` (`qdl`); verify both paths exist
- [x] 1.3 Update `image/os/buildroot/build.sh` usage board list and
      `image/os/README.md` boards table to include `qdl`; verify docs mention
      `build.sh qdl`

## 2. Kernel config

- [x] 2.1 Author minimal `board/obake/qdl/linux.config` for Quieter DL
      (NVMe, IGC, PTP/PHY timestamping/XDP, HDA, USB MIDI 2.0 + UMP legacy
      rawmidi, host-contract EFI/loop/SquashFS options; no wireless; no
      i915 under PREEMPT_RT; no PREEMPT_RT/NO_HZ in board file) and verify
      the file is a short curated config rather than a quieter paste

## 3. Build verification

- [x] 3.1 Run `image/os/buildroot/build.sh qdl` headlessly and verify the
      build completes with `/etc/obake/board` reporting `qdl`
- [x] 3.2 Assert the built kernel `.config` has `CONFIG_PREEMPT_RT=y`,
      `CONFIG_NO_HZ_IDLE=y`, `CONFIG_SND_USB_AUDIO_MIDI_V2=y`,
      `CONFIG_IGC=m`, and wireless drivers unset

## 4. Pinned inputs

- [x] 4.1 Complete `image/os/buildroot/vendor/dl/` (git-lfs) with the sources
      the shared base and x86_64 EFI path require but the aarch64-derived
      snapshot lacked (`gptfdisk`, `dosfstools`, `parted`, `lvm2`,
      `go` linux-amd64, `gnu-efi`, `python-pyelftools`) and verify a qdl build
      resolves them from the cache

## 5. Shared package fix

- [x] 5.1 Add `package/singularity/0002-go-disable-vcs-stamping.patch`
      (`-buildvcs=false`) so the Singularity Go build does not fail under
      Buildroot's `GIT_DIR=.`, and verify the qdl image build completes
