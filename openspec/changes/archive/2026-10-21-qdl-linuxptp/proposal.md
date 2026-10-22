# Proposal

## Why

A Quieter DL that can timestamp PTP in hardware still has no `ptp4l` on the
host. The `ptp-tools` shape can run the suite, but only inside a container
that is handed the NIC, `/dev/ptp*`, and clock capabilities. `qdl` should
be able to run that suite as ordinary system services, with a configuration
the `obake` user can edit, and with those services left stopped until the
operator turns them on.

## What Changes

- The `qdl` image installs the linuxptp 4.4 userspace suite: `ptp4l`,
  `phc2sys`, `pmc`, `phc_ctl`, `hwstamp_ctl`, `timemaster`, `ts2phc`, `nsm`,
  and `tz2alt`. Other boards do not.
- `ptp4l.service` and `phc2sys.service` are system units. Neither is
  enabled, and neither starts at boot. `ptp4l` does not pull `phc2sys` in.
- Each unit runs as uid `obake` and receives only the capabilities that
  service needs. The rest of the `obake` session does not gain them.
- The live `ptp4l` configuration and the `phc2sys` options live under
  `/home/obake/.config/linuxptp/`. The `obake` user can edit those files.
  A first-boot seed creates them only when they are absent.
- The `obake` user starts, stops, enables, disables, and reads logs for
  these units with the existing polkit and journal grants.
- The `ptp-tools` shape stays as the portable kit for other hosts.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `os-image-build`: the `qdl` image ships the linuxptp userspace suite and
  disabled system units whose configuration the `obake` user can edit.

## Impact

- `qdl` Buildroot config only. The shared base does not gain the package.
- The linuxptp 4.4 source tarball joins the offline download cache.
- New `qdl` rootfs files: the two unit files, a PHC device rule, the
  config seed, and a tmpfiles snippet. The user guide notes how to enable
  the services.
- OTA delivers the binaries and the disabled units. An existing
  `/home/obake` tree is left alone except for the one-time seed of a
  missing config. Nothing enables the units. No persist-partition
  migration.
- Hardware timestamps on `qdl` still depend on the `r8125` PHC. These
  tools can be installed before that image is on the board; a hardware
  lock cannot.
- QEMU cannot lock PTP. The `ptp-tools` shape is unchanged, including its
  sample AES67 configs.
