# Proposal

## Why

The image already builds `systemd-timesyncd`, but the unit is not enabled, so a
fresh host never sets its system clock from the network. TLS, logs, and
certificate checks then run against a wrong or unset time after power loss or
on boards without a reliable RTC.

## What Changes

- Enable `systemd-timesyncd` by default on every board image (same overlay
  wants pattern used for networkd/resolved).
- Keep stock NTP defaults (no custom server list); do not gate boot on a
  successful sync (`systemd-time-wait-sync` stays disabled).
- Document network time sync in host config notes, including that operators
  who enable `phc2sys` for PTP should stop/disable timesyncd so the two do
  not fight over the system clock.
- Extend guest verification so a disabled timesyncd fails the release check.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `host-config`: require that the image enables `systemd-timesyncd` so the
  system clock is synchronized from the network when connectivity allows.
- `image-verification`: after boot, confirm `systemd-timesyncd` is enabled
  (and active) so regressions are caught headlessly.

## Impact

- `image/os/buildroot/board/obake/rootfs-overlay/etc/systemd/system/sysinit.target.wants/`
  (add `systemd-timesyncd.service` wants symlink)
- Optionally pin `BR2_PACKAGE_SYSTEMD_TIMESYNCD=y` in
  `image/os/buildroot/configs/obake_base.config` for explicitness
- `image/docs/host-config.md` network/tooling section
- `image/verification/guest/obake-verify.sh`
- Persist: `/var/lib/systemd` is already in the persisted `/var` subset, so
  timesyncd's clock file survives reboot/slot switch once the unit runs
- OTA: existing hosts pick up the enabled unit on the next image apply; no
  persist-seeded network file rewrite
- PTP (`qdl` `phc2sys`): remains opt-in and stopped by default; docs note the
  mutual exclusion with NTP when both would discipline the system clock
