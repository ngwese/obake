# Design

## Context

See proposal.md for why `qdl` gets the linuxptp suite. Constraints that
shape the approach:

- Buildroot 2026.02.3 already packages linuxptp 4.4, the same version the
  `ptp-tools` shape pins. Its stock units run as root, bind a section
  named `eth0`, and `ptp4l.service` pulls in `phc2sys`.
- `BR2_PACKAGE_LINUXPTP` is selected only from a board fragment.
  `configs/obake-qdl.config` already points
  `BR2_ROOTFS_OVERLAY` at `board/obake/qdl/rootfs-overlay`, which is
  copied after packages install.
- systemd 258 starts `systemd --user` as uid `obake` with no
  capabilities. `AmbientCapabilities=` in a user unit cannot grant
  `CAP_NET_ADMIN` or `CAP_SYS_TIME`. A system unit can, because pid 1
  holds them.
- `/etc` is not writable by the `obake` login. `/home` is the user
  partition, mounted before `systemd-tmpfiles-setup.service`, and it
  survives an image update. `49-obake.rules` already allows that login
  to manage every system unit and to read the system journal.
- Hardware timestamps require the `r8125` PHC from `qdl-r8125-hwts`.
  This change does not wait on that, and it does not prove a lock.

## Goals / Non-Goals

**Goals:**

- Install the whole linuxptp 4.4 userspace suite on `qdl` only.
- Run `ptp4l` and `phc2sys` as system services, uid `obake`, stopped
  until the operator enables them.
- Keep the live configuration on the user partition, seeded once.

**Non-Goals:**

- Enabling either unit in the image, or making `ptp4l` start `phc2sys`.
- File capabilities on the binaries.
- Installing `ethtool` or `tcpdump` on the host. The shape still has them.
- Changing the `ptp-tools` shape, its pin, or its sample configs.
- Choosing the studio interface. That is per machine, in the seeded files.
- A QEMU PTP lock, or an image-version bump.

## Decisions

### 1. Buildroot's linuxptp package, `qdl` only

Select `BR2_PACKAGE_LINUXPTP=y` in `configs/obake-qdl.config` and nowhere
else. The package builds linuxptp 4.4 and installs every program from
`make install` into `/usr/sbin`.

The tarball is not in `image/os/buildroot/vendor/dl` yet. Add
`linuxptp-4.4.tgz` there so an offline build hits the existing Git LFS
rule for `os/buildroot/vendor/dl/**`. The hash in the Buildroot package
already matches that release.

Alternative: a from-scratch recipe. Rejected. The version, hash, and
build are already pinned upstream of us.

### 2. System units with ambient capabilities, not the user slice

Replace the package's unit files from the `qdl` rootfs overlay. The
overlay is last, so it wins over the package.

`ptp4l.service`:

- `User=obake`, `Group=obake`
- `AmbientCapabilities=` and `CapabilityBoundingSet=` limited to
  `CAP_NET_BIND_SERVICE`, `CAP_NET_ADMIN`, and `CAP_SYS_TIME`
- `NoNewPrivileges=yes`
- `ExecStart=/usr/sbin/ptp4l -f /home/obake/.config/linuxptp/ptp4l.cfg -m`
- `WantedBy=multi-user.target`, no `Wants=phc2sys.service`
- not enabled. The image build runs `systemctl preset-all`, whose
  default is to enable a unit that has an `[Install]` section, so a
  `qdl` preset `50-obake-linuxptp.preset` disables both units. The
  operator's later `systemctl enable` still creates the wants symlink.

`phc2sys.service`:

- same user
- ambient and bounding set limited to `CAP_SYS_TIME`
- `EnvironmentFile=/home/obake/.config/linuxptp/phc2sys.env`
- `ExecStart=/usr/sbin/phc2sys $PHC2SYS_OPTS`
- `After=ptp4l.service` without `Requires=` or `BindsTo=`
- same install section, also not enabled

UDP AES67 does not need `CAP_NET_RAW`. Neither unit gets
`CAP_DAC_OVERRIDE`. No `PrivateUsers=`, which would drop host
capabilities.

A udev rule in the same overlay sets `KERNEL=="ptp[0-9]*"` to
`OWNER="obake"`, `GROUP="obake"`, `MODE="0660"`, so the service can open
the PHC. No new group, and other boards do not get the rule.

Alternative considered: user units plus `setcap` on the binaries. The
user manager cannot grant the capabilities, and file capabilities would
let any `obake` process step the clock and bind ports 319 and 320.
Passwordless sudo already covers one-shot tools (`hwstamp_ctl`,
`phc_ctl`) from a login shell.

### 3. Configuration lives in the home directory

Templates ship in the `qdl` overlay under
`/usr/share/obake/linuxptp/`. A `qdl` tmpfiles snippet uses type `C`
so each destination is copied only when missing:

- `/home/obake/.config/linuxptp/ptp4l.cfg`
- `/home/obake/.config/linuxptp/phc2sys.env`

Both are mode `0644`, owner `obake`. `systemd-tmpfiles-setup.service`
runs after `local-fs.target`, which is when `/home` is mounted.

`ptp4l.cfg` follows `shapes/ptp-tools/configs/aes67-client.cfg`:
`clientOnly`, priorities 255, domain 0, UDPv4, E2E, hardware
timestamping, and the AES67 intervals. The interface section header is
the placeholder `REPLACE_WITH_INTERFACE`, not `eth0` and not a name the
image creates. `phc2sys.env` sets

```
PHC2SYS_OPTS=-s REPLACE_WITH_INTERFACE -c CLOCK_REALTIME -w -r
```

`-r` slews. There is no `-S`, so the seed does not step
`CLOCK_REALTIME`. The operator edits both files, then enables the units.

Replace `/etc/linuxptp.cfg` from the overlay with a short comment that
points at the home directory, so the package's `eth0` client is not a
second configuration.

### 4. Leave the shape and the other boards alone

No edit under `shapes/ptp-tools/`. No `BR2_PACKAGE_LINUXPTP` in
`obake_base.config`. `check-boards.sh` already fails a board fragment
that drifts the shared base; the new symbol stays in the `qdl` fragment.

## Risks / Trade-offs

- [Stock unit survives] → the `qdl` overlay is the second
  `BR2_ROOTFS_OVERLAY` entry and replaces the same paths the package
  installs. The headless check compares the installed unit text.
- [Seed lands on the rootfs mount of `/home` and is then hidden] →
  tmpfiles run after local filesystems. The seed task checks the
  ordering rather than adding a new mount unit.
- [Operator enables the seed unchanged] → `REPLACE_WITH_INTERFACE` fails
  the service instead of joining a real LAN or stepping the clock.
- [Two RTL8125 ports, two PHC devices] → the operator names the AES67
  port in both files. The udev rule covers every `ptpN` node.
- [phc2sys enabled alone] → `-w` waits for `ptp4l` and does not step
  the clock in the meantime.
- [Rollback after enable] → the wants symlink lives in the persistent
  `/etc` overlay. An older image with no such unit ignores the dangling
  symlink. The home config remains and is unused.
- [Hardware timestamping still unverified on a Quieter DL] → the suite
  installs either way. A lock is a board test, not a QEMU gate.

## Migration Plan

An image update adds the binaries, the disabled units, the udev rule,
and the templates. First boot copies a missing config into
`/home/obake/.config/linuxptp/` and does not enable either unit.
Rollback is the previous image. No persist-partition migration.

## Open Questions

None.
