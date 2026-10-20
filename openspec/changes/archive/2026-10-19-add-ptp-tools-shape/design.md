# Design

## Context

See proposal.md for motivation. Constraints that shape the approach:

- `shapes/pipewire-tools/` is the precedent for a minimal Alpine toolkit
  shape: single-stage `alpine:edge`, `apk add`, `%runscript` → `exec "$@"`,
  shape labels, host wrapper script, README ladder.
- Host kernels already expose PTP / PHY timestamping plumbing (AOIP path);
  AES67/RAVENNA audio is not installed on the host yet. This shape is for
  bring-up and learning, not a production clock service.
- Alpine packages `linuxptp` under **edge/testing** (metapackage pulling
  `ptp4l`, `phc2sys`, `pmc`, `phc_ctl`, `hwstamp_ctl`, and related tools).
  `ethtool`, `tcpdump`, and `iproute2` are in the usual Alpine repos.
- Singularity does not grant host network, `/dev/ptp*`, or `CAP_NET_ADMIN` /
  `CAP_SYS_TIME` by default; a wrapper must pass those for HWTS and PTP.

## Goals / Non-Goals

**Goals:**

- One small SIF with the HWTS/PTP operator toolset and AES67/RAVENNA sample
  configs.
- Clear docs: probe NIC → lock AES67 client → prove with `pmc`/`tcpdump` →
  optional `phc2sys`.
- Same packaging pattern as `pipewire-tools` (Alpine, labels, wrapper).

**Non-Goals:**

- Host systemd units for `ptp4l` / `phc2sys`, or persistent studio lock at boot.
- AES67/RAVENNA audio (Merging, PipeWire AES67, Ravenna kernel modules).
- SMPTE 2059, gPTP/802.1AS, or other non-AES67 PTP profiles.
- Wireshark/`tshark`, chrony/`timemaster`, or GPS/`ts2phc` unless later needed.
- Putting these packages on the host rootfs.

## Decisions

### Single-stage Alpine edge with testing enabled

Bootstrap `alpine:edge`. In `%post`, enable the `testing` repository (needed
for `linuxptp` today), then:

```
apk add --no-cache ethtool linuxptp tcpdump iproute2
```

Install the `linuxptp` metapackage so subpackages stay consistent. Alternative
considered: Debian slim with `linuxptp` from apt. Rejected to match the
requested minimal Alpine toolkit pattern and `pipewire-tools`.

### App version pin tracks linuxptp

`com.afofo.obake.shape-app-version` is the Alpine `linuxptp` version (e.g.
`4.4`), templated via `{{ linuxptp_version }}` / `build-args.txt` like
`pipewire-tools`. Bumping Alpine edge or the package is a deliberate pin
change.

### Sample configs under `shapes/ptp-tools/configs/`

Ship four files (names may match these intents):

| File | Intent |
|------|--------|
| `hwts-client.cfg` | Minimal HWTS client, AES67 transport defaults |
| `swts-client.cfg` | Same with software timestamping |
| `aes67-client.cfg` | Domain 0, E2E, UDPv4, DSCP 46/34, media intervals, `clientOnly 1`, high `priority1` |
| `lab-gm.cfg` | Scratch-LAN GM only |

Configs omit a hard-coded interface; operators pass `-i <iface>` on the
`ptp4l` command line. Bake configs into the SIF (e.g. `/opt/ptp-tools/configs`)
and keep the same tree in the git source for reading without a build.

Alternative: only document flags in the README. Rejected; checked-in configs
are easier to iterate and less error-prone for AES67 intervals.

### Wrapper mirrors pipewire-tools, different privileges

Provide `shapes/ptp-tools/ptp-tools` that locates the SIF (`PTP_TOOLS_SIF` or
common build paths) and runs with host network, PTP device nodes, and
capabilities needed for HWTS / clock control, for example:

- `--net` (host network namespace)
- bind or map `/dev/ptp*` (and document `ethtool`/`hwstamp_ctl` needs)
- `--cap` / fakeroot vs privileged as required by Singularity on the host

Exact flags follow what Apptainer/Singularity on the target host accepts;
document the working invocation and keep the wrapper the supported path.
Do not use a PipeWire runtime-dir bind.

### Docs: AES67 ladder and lab-GM warning

README covers build, wrapper usage, and a short ladder:

1. `ethtool -T $IF` / `hwstamp_ctl` / `phc_ctl`
2. `ptp4l -f …/aes67-client.cfg -i $IF -m`
3. `pmc` dataset queries
4. optional `phc2sys`
5. `tcpdump` on UDP 319/320 with adapter timestamps when useful

Explicitly warn: never run `lab-gm.cfg` on a studio PTP network.

## Risks / Trade-offs

- [Alpine `linuxptp` only in testing] → Enable testing in the def; pin the
  package version in the label; if testing drops or breaks, rebuild from a
  known-good Alpine snapshot or vendor a static build in a later change.
- [RTL8125 / driver may lack usable HWTS] → Ship `swts-client.cfg` so
  AES67 client experiments can continue while HWTS is debugged.
- [Privileged wrapper on a shared host] → Document required caps; toolkit is
  for trusted operators, not unprivileged multi-tenant use.
- [Accidental GM on studio LAN] → `clientOnly` + high priority on client
  samples; lab GM called out as scratch-only in docs and comments.

## Migration Plan

1. Land definition, configs, wrapper, README, and shapes index row.
2. Build the SIF with Singularity/Docker bootstrap; verify tool presence and
   labels.
3. On a host with a suitable NIC, run the HWTS probe steps; optionally lock
   to a lab GM or studio AES67 GM.
4. No host rollback; stop using the SIF if unwanted.

## Open Questions

None.
