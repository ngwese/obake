# Tasks

## 1. Writable `/media` mount root

- [x] 1.1 Add a tmpfs entry for `/media` in
      `image/os/buildroot/board/obake/rootfs-overlay/etc/fstab` (or an
      equivalent `.mount` unit under the overlay) with
      `mode=0755,nosuid,nodev,noexec`. Verify: fstab/unit present; staged
      target mounts `/media` as tmpfs after boot in QEMU or on hardware
      (`findmnt -T /media`).
- [x] 1.2 Update
      `image/os/buildroot/board/obake/rootfs-overlay/etc/obake/directory-contract`
      and `image/docs/host-config.md` so `/media` is named as the volatile
      removable-media mount root. Verify: contract and docs mention `/media`
      as volatile; no implication that `/media` is a plain RO-root directory.

## 2. Host-namespace automount path

- [x] 2.1 Add a systemd oneshot template (e.g.
      `obake-media-automount@.service`) under the rootfs overlay that runs
      `/usr/libexec/obake/media-automount` add/remove for `%i` in the host
      mount namespace. Verify: unit file installed under
      `usr/lib/systemd/system/` (or overlay equivalent); `systemctl cat`
      shows the helper as ExecStart.
- [x] 2.2 Change
      `etc/udev/rules.d/99-obake-media-automount.rules` to request the
      oneshot via `TAG+="systemd"` / `ENV{SYSTEMD_WANTS}` (or
      `systemd-mount`) instead of `RUN+` calling `mount` directly; match
      `ACTION=="add|change"` for filesystem partitions and keep USB /
      removable filters. Verify: rule file has no `RUN+=... media-automount`
      mount path; wants/unit handoff present; `add|change` matched.
- [x] 2.3 Update `/usr/libexec/obake/media-automount` to no-op when the
      device is already mounted at the chosen path, and to surface mkdir/
      mount failures to the journal (stop silent `2>/dev/null` on those
      errors). Verify: re-running add for an already-mounted device exits
      0 without remount errors; a forced failure appears in the journal.

## 3. Verification and docs

- [x] 3.1 Strengthen `check_media_automount` in
      `image/verification/guest/obake-verify.sh` so it fails unless `/media`
      is a mountpoint and a privileged probe can create and remove a
      directory under `/media` (keep helper/rule presence checks). Verify:
      check fails on a RO `/media` directory; passes when tmpfs `/media` is
      mounted in guest functional runs.
- [x] 3.2 Confirm `image/docs/user-guide.md` and `image/docs/os-update.md`
      still describe `/media/<label>` automount and do not imply manual
      mountpoint creation on the RO root. Verify: docs consistent with
      host-config wording from 1.2.
- [x] 3.3 Headless integration: rebuild/run the existing QEMU guest
      functional path for a board that exercises `obake-verify.sh` and
      confirm the media-automount check passes. Note hardware USB insert as
      a manual follow-up outside the harness. Verify: guest suite reports
      media-automount pass; no regression in other guest checks.
