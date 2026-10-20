# Tasks

## 1. Shape definition and sample configs

- [x] 1.1 Add `shapes/ptp-tools/ptp-tools.def` as a single-stage Alpine
  Singularity definition that enables the testing repo, installs `ethtool`,
  `linuxptp`, `tcpdump`, and `iproute2` via `apk`, sets `%runscript` to
  `exec "$@"`, does not start a PTP daemon, and sets shape labels with
  `com.afofo.obake.shape-name` `ptp-tools` and
  `com.afofo.obake.shape-app-version` `{{ linuxptp_version }}`. Add
  `shapes/ptp-tools/build-args.txt` pinning the Alpine linuxptp version.
  Verify by reading the def: Alpine base, testing enabled, packages listed,
  no daemon startscript, and the three shape labels present.
- [x] 1.2 Add AES67/RAVENNA-only sample configs under
  `shapes/ptp-tools/configs/` (`hwts-client.cfg`, `swts-client.cfg`,
  `aes67-client.cfg`, `lab-gm.cfg`) and copy them into the image at a path
  documented in the README (e.g. `/opt/ptp-tools/configs`). Verify each
  client uses domain 0 / E2E / UDPv4 as appropriate, clients set
  `clientOnly`, and no file configures SMPTE 2059 or gPTP.
- [x] 1.3 Add `shapes/ptp-tools/ptp-tools` host wrapper (locate SIF via
  `PTP_TOOLS_SIF` or common build paths; host network; `/dev/ptp*` access;
  capabilities needed for HWTS/clock ops) and `shapes/ptp-tools/README.md`
  with build, wrapper usage, the HWTS→AES67 client→pmc→phc2sys ladder, and
  a scratch-LAN-only warning for `lab-gm.cfg`. Add a `ptp-tools` row to
  `shapes/README.md`. Verify the docs name the required tools and the lab-GM
  warning, and that the wrapper script is executable.

## 2. Headless verification

- [x] 2.1 Build the SIF with Singularity (`Bootstrap: docker`) and verify
  `singularity exec <sif> sh -c 'command -v ethtool ptp4l phc2sys pmc phc_ctl hwstamp_ctl tcpdump ip'`
  succeeds for each tool, sample configs exist at the documented image path,
  and image labels report shape-name `ptp-tools` with app version matching
  `build-args.txt`.
