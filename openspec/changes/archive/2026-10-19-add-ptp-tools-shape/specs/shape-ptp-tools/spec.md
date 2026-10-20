# Spec Delta

## Purpose

Provides a portable Singularity shape with PTP and hardware-timestamping
tools plus AES67/RAVENNA sample `ptp4l` configs so an operator can bring up
NIC HWTS and experiment with locking to a studio grandmaster without installing
those packages on the host rootfs.

## ADDED Requirements

### Requirement: Alpine PTP tools shape image
The repository SHALL provide a Singularity definition at
`shapes/ptp-tools/ptp-tools.def` that builds from a minimal Alpine base and
installs `ethtool`, the Alpine `linuxptp` package set, `tcpdump`, and
`iproute2`. The built image SHALL include at least `ethtool`, `ptp4l`,
`phc2sys`, `pmc`, `phc_ctl`, `hwstamp_ctl`, `tcpdump`, and `ip`. The image
SHALL carry the standard shape labels `com.afofo.obake.shape-schema`,
`com.afofo.obake.shape-name` set to `ptp-tools`, and
`com.afofo.obake.shape-app-version` set to the installed linuxptp package
version.

#### Scenario: Built image contains the toolset
- **WHEN** the `ptp-tools` shape is built into a SIF
- **THEN** `ethtool`, `ptp4l`, `phc2sys`, `pmc`, `phc_ctl`, `hwstamp_ctl`,
  `tcpdump`, and `ip` are present as executable commands inside the image
- **AND** the image labels identify the shape as `ptp-tools` with an app
  version matching the Alpine linuxptp package

### Requirement: Operator toolkit without host PTP service
The shape SHALL NOT start a persistent host PTP or clock-sync service and
SHALL NOT install an AES67 or RAVENNA audio stack. Its runscript SHALL forward
arguments to a command inside the image (`exec "$@"`) so the operator chooses
which tool to run.

#### Scenario: Runscript forwards a one-shot tool
- **WHEN** an operator runs the shape with arguments `ethtool --version`
- **THEN** the container executes `ethtool --version` and exits
- **AND** no systemd unit or background PTP daemon is started by the shape
  definition

### Requirement: AES67 and RAVENNA sample ptp4l configs
The shape directory SHALL ship sample `ptp4l` configuration files scoped to
AES67/RAVENNA PTP only: a minimal hardware-timestamping client, a software-
timestamping client for comparison, a fuller AES67 client (domain 0, E2E,
UDPv4, media-oriented intervals, `clientOnly`), and a scratch-LAN lab
grandmaster. Those samples SHALL NOT define SMPTE 2059, gPTP, or other
non-AES67 media profiles.

#### Scenario: Sample configs are present in the tree
- **WHEN** an operator inspects `shapes/ptp-tools/configs/`
- **THEN** they find client configs for hardware and software timestamping,
  an AES67 client config, and a lab grandmaster config
- **AND** none of those files configure a non-AES67 media profile such as
  SMPTE 2059 or gPTP

### Requirement: Documented build and run
`shapes/README.md` SHALL list the `ptp-tools` shape. The shape directory SHALL
document how to build the SIF and how to run tools against the host NIC and
PTP devices, including host network access, `/dev/ptp*` visibility, and the
capabilities needed for hardware timestamping and clock adjustment. Docs SHALL
include a short bring-up ladder from `ethtool -T` through an AES67 client lock
and warn that the lab grandmaster config is for a scratch LAN only.

#### Scenario: Operator finds AES67 PTP bring-up instructions
- **WHEN** an operator opens `shapes/README.md` and the `ptp-tools` shape
  documentation
- **THEN** they can build the image and invoke tools such as `ethtool`,
  `ptp4l`, and `pmc` against a host interface without reading a host recipe
- **AND** the docs warn not to run the lab grandmaster config on a studio
  PTP network
