# Proposal

## Why

The host PipeWire graph from `host-pipewire` can be inspected and exercised
with upstream tools (`pw-cat` / play / record, `pw-top`, `pw-profiler`, and
the rest of the PipeWire CLI). Those tools are not all on the host image
(`pw-cat` and `pw-top` are off or missing). A small Singularity shape gives
operators the full toolset without growing the host rootfs, and Alpine keeps
the image minimal while shipping PipeWire 1.6.9 to match the host daemon.

## What Changes

- Add a new shape `shapes/pipewire-tools/` whose Singularity definition is
  based on a minimal Alpine image and installs Alpine's `pipewire-tools`
  package (and its runtime dependencies only).
- The shape is a client of the host PipeWire socket. It does not start a
  PipeWire or WirePlumber daemon inside the container.
- Document how to build and run the shape against `/run/user/<uid>/pipewire-0`
  (bind-mount and `XDG_RUNTIME_DIR`), and list it in `shapes/README.md`.
- Pin the shape app version to the Alpine PipeWire package version so the
  image label tracks the toolset.

## Capabilities

### New Capabilities

- `shape-pipewire-tools`: a Singularity shape that provides the PipeWire CLI
  toolset as a portable client of the host graph.

### Modified Capabilities

None.

## Impact

- New `shapes/pipewire-tools/pipewire-tools.def` and a short README under that
  directory (or a short section linked from `shapes/README.md`).
- No host image, OTA, or persistent configuration change. The shape runs only
  on a host that exposes a PipeWire user socket compatible with the Alpine
  client (the 1.6.9 host from `host-pipewire` matches Alpine edge's
  `pipewire-tools` 1.6.9).
