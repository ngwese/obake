# Design

## Context

See proposal.md for motivation. Constraints that shape the approach:

- Existing shapes use multi-stage Debian Bookworm (or planned Trixie) defs
  under `shapes/<name>/<name>.def` with `com.afofo.obake.shape-*` labels.
- `host-pipewire` runs PipeWire **1.6.9** for user `obake`. The socket is
  `/run/user/<uid>/pipewire-0`. Singularity does not bind `/run/user` by
  default.
- The host omits `pw-cat` (`-Dpw-cat=disabled`) and typically omits `pw-top`
  (no ncurses). `pw-profiler` may be present on the host; the shape still
  bundles the full Alpine toolset for a portable operator image.
- Alpine's `community/pipewire-tools` on **edge** is **1.6.9-r0** and
  provides `pw-cat`, `pw-play`, `pw-record`, `pw-top`, `pw-profiler`,
  `pw-cli`, and the other upstream CLI tools. It depends on `pipewire`
  libraries (and ncurses, readline, sndfile), not on a running daemon in
  the container.
- Alpine is musl; the host is glibc. PipeWire clients talk over the Unix
  socket protocol, so a musl client against a glibc host daemon is fine
  when versions match.

## Goals / Non-Goals

**Goals:**

- One small SIF that is only the PipeWire CLI toolset.
- Version alignment with the host daemon (1.6.9).
- Clear run instructions that reuse the same runtime-dir bind as JACK
  client shapes.

**Non-Goals:**

- Putting these tools on the host image.
- JACK compatibility libraries (`pipewire-jack`) or ALSA plugin packages
  inside this shape.
- Starting PipeWire/WirePlumber in the container.
- Changing `trixie-jack-shapes` or application shapes (chuck, rnbo-runner).
- Floating forever on unpinned Alpine edge without a recorded package
  version in the shape label.

## Decisions

### Single-stage Alpine definition

No compile stage. Bootstrap from Docker `alpine` (prefer a tag whose
`community` repo ships PipeWire 1.6.9; today that is `alpine:edge`, or a
future stable release that catches up). `%post` enables `community` if
needed, runs `apk add --no-cache pipewire-tools`, and records the installed
`pipewire` package version into the shape label (template or `apk` query
written into a file the labels section references).

Alternative considered: Debian Trixie slim with `pipewire-bin`. Rejected
because the request is a minimal Alpine base and Alpine already matches
host 1.6.9 while Trixie is 1.4.2.

### Package is `pipewire-tools` only

Install `pipewire-tools` and let apk pull runtime shared libraries. Do not
install `pipewire-pulse`, session-manager packages, or OpenRC/systemd unit
packages meant to run a daemon. Do not add a `%startscript` that launches
`pipewire`.

### Runscript forwards arguments

`%runscript` is `exec "$@"`. Operators run:

```
singularity run pipewire-tools.sif pw-top
singularity run pipewire-tools.sif pw-cat --help
```

or `singularity exec … pw-profiler`. Document the required binds:

```
--bind /run/user/$(id -u) \
--env XDG_RUNTIME_DIR=/run/user/$(id -u)
```

Same pattern as the planned JACK client guide.

### Docs under the shape and the shapes index

`shapes/pipewire-tools/README.md` holds build and run. `shapes/README.md`
gets a table row. No new host docs required; a one-line pointer from
`shapes/docs/pipewire-jack.md` (when that guide lands) is optional and not
required by this change.

### App version pin

`com.afofo.obake.shape-app-version` is the Alpine PipeWire version string
(e.g. `1.6.9`). Bumping the Alpine base or the apk package is a deliberate
pin change in a later commit, not an unattended float of `latest` without
updating the label.

## Risks / Trade-offs

- [Alpine edge floats] → Record the package version in the label; when
  rebuilding, confirm `apk` still installs 1.6.x compatible with the host,
  or pin an Alpine image digest / package version in the def.
- [musl vs glibc surprises] → Unlikely for the core protocol; if a tool
  fails, fall back to host-native `pw-cli` / `pw-profiler` for that case.
- [apk pulls a `pipewire` package with unit files] → Units are inert under
  Singularity; do not enable or invoke them. Verify no `%startscript`
  starts a daemon.

## Migration Plan

1. Land the definition, README, and shapes index row.
2. Build the SIF on a machine with Singularity and Docker bootstrap.
3. On a host with `host-pipewire` applied, run `pw-cli info` through the
   shape with the runtime-dir bind; confirm `pipewire-0`.
4. No host rollback path; remove or stop using the SIF if unwanted.

## Open Questions

None.
