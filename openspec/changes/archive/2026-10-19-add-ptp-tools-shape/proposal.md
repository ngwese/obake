# Proposal

## Why

Hardware timestamping and AES67/RAVENNA PTP bring-up need userspace tools
(`ethtool`, linuxptp, packet capture) that do not belong on the minimal host
rootfs. A small Alpine Singularity shape gives operators a portable kit to
probe NIC HWTS, lock a PTP client to a studio grandmaster (or a lab GM), and
learn the AES67 media profile without installing those packages on every host.

## What Changes

- Add a new shape `shapes/ptp-tools/` based on a minimal Alpine image that
  installs `ethtool`, the Alpine `linuxptp` suite, `tcpdump`, and `iproute2`.
- Ship sample `ptp4l` configs for AES67/RAVENNA PTP only: HWTS and SWTS
  clients, a fuller AES67 client, and a scratch-LAN lab GM. No SMPTE 2059,
  gPTP, or other media profiles.
- Provide a thin host wrapper and docs for build/run (host network, `/dev/ptp*`,
  needed capabilities), and list the shape in `shapes/README.md`.
- The shape is an operator toolkit: it does not start a persistent PTP daemon
  on the host, and does not install AES67/RAVENNA audio stacks.

## Capabilities

### New Capabilities

- `shape-ptp-tools`: a Singularity shape that provides PTP and HWTS debugging
  tools plus AES67/RAVENNA sample `ptp4l` configs as a portable operator kit.

### Modified Capabilities

None.

## Impact

- New `shapes/ptp-tools/` (definition, wrapper, sample configs, README) and a
  row in `shapes/README.md`.
- No host image, OTA, or persistent configuration change. Cross-host
  portability is via the SIF, matching other toolkit shapes such as
  `pipewire-tools`.
- Alpine currently packages `linuxptp` in `edge/testing`; the definition must
  enable that repository. Package version is pinned in the shape app-version
  label.
