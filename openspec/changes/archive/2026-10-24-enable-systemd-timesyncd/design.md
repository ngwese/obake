# Design

## Context

See proposal.md for why. Current state that shapes the approach:

- Buildroot defaults already set `BR2_PACKAGE_SYSTEMD_TIMESYNCD=y` (and
  `TIMEDATED`). The binary and unit ship in the rootfs.
- systemd's `90-systemd.preset` says `enable systemd-timesyncd.service`, and
  the package install runs `systemctl --root=… preset-all`. Despite that, the
  built image today reports the unit **disabled**: there is no
  `sysinit.target.wants/systemd-timesyncd.service` link. Obake already enables
  networkd/resolved/avahi via overlay wants symlinks under
  `etc/systemd/system/…target.wants/`; timesyncd has no such link.
- `WantedBy=sysinit.target` on the stock unit. The overlay already has
  `sysinit.target.wants/` for `obake-persist-var.service`.
- `/var/lib/systemd` is already listed in `etc/obake/persisted-var`, so
  timesyncd's `StateDirectory=systemd/timesync` survives reboot once the
  unit runs.
- `qdl` ships opt-in `phc2sys` (disabled by preset) which can also discipline
  the system clock. Default boot leaves it stopped.
- `systemd-time-wait-sync` is disabled by the vendor preset; uncabled /
  offline boot must stay non-blocking.

## Goals / Non-Goals

**Goals:**

- Make timesyncd enabled and active on every board image by default.
- Match the existing overlay wants pattern so enablement is explicit and
  survives rebuild quirks around presets.
- Fail guest verification if the unit is not enabled/active.
- Document NTP sync and the PTP mutual-exclusion note for operators.

**Non-Goals:**

- Custom NTP server lists or DHCP-NTP-only policy (keep stock
  `timesyncd.conf` / compile-time FallbackNTP).
- Enabling `systemd-time-wait-sync` or otherwise delaying boot on sync.
- Auto-disabling timesyncd when the operator enables `phc2sys` (document
  only).
- Changing RTC hardware support, `fake-hwclock`, or chrony.
- Image version / pin bumps beyond what a normal rebuild already records.

## Decisions

### 1. Overlay wants symlink under `sysinit.target.wants`

Add:

`etc/systemd/system/sysinit.target.wants/systemd-timesyncd.service`
→ `/usr/lib/systemd/system/systemd-timesyncd.service`

Same pattern as `multi-user.target.wants/systemd-networkd.service` et al.
This does not depend on `preset-all` succeeding or on overlay merge order
preserving a preset-created link.

Alternative: rely on `systemctl preset-all` alone. Rejected — that path
already left the unit disabled in a current build.

Alternative: post-build `systemctl --root enable`. Rejected — overlay wants
are the established, reviewable mechanism for always-on host services.

### 2. Explicitly set `BR2_PACKAGE_SYSTEMD_TIMESYNCD=y` in `obake_base.config`

Pin the package beside `SYSTEMD_NETWORKD` / `RESOLVED` so a future Buildroot
default flip cannot silently drop NTP. No other timesyncd Buildroot options
are needed.

### 3. Verification checks enablement and activity, not sync success

Guest check: `systemctl is-enabled` and `is-active` for
`systemd-timesyncd.service`. Do not assert `timedatectl` NTPSynchronized —
QEMU/sandbox may block UDP/123. Mirrors how DNS already falls back when
outbound resolution fails.

### 4. Docs note for PTP operators

In `image/docs/host-config.md` network section: timesyncd is on by default;
if enabling `phc2sys` to discipline the system clock from a PHC, stop and
disable timesyncd first. No automatic conflict resolution in this change.

## Risks / Trade-offs

- [NTP and PTP both active] → Mitigation: phc2sys stays disabled by default;
  docs warn operators who enable it.
- [Wrong time until first sync / offline hosts] → Mitigation: accept
  asymptotic sync; do not block boot. Persisted clock file under
  `/var/lib/systemd/timesync` improves cold-boot estimates after the first
  successful sync.
- [Public FallbackNTP (compile-time Google pools)] → Mitigation: acceptable
  for headless studio hosts with outbound DNS; operators can drop in
  `/etc/systemd/timesyncd.conf.d/` via the persist overlay later if needed.
  Out of scope to customize now.

## Migration Plan

- New images: timesyncd enabled out of the box.
- Existing hosts: next OTA apply brings the overlay wants link on the new
  root slot; no persist rewrite required.
- Rollback: previous slot lacks the wants link; unit disabled again after
  revert.
- Operators already running custom NTP (chrony, etc.): rare on stock image;
  they must mask or disable timesyncd themselves if they install a competing
  daemon later.
