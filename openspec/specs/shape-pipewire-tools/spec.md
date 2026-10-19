# shape-pipewire-tools Specification

## Purpose

Provides a portable Singularity shape with the PipeWire CLI toolset so an
operator can inspect and exercise the host PipeWire graph without installing
those tools on the host rootfs.

## Requirements

### Requirement: Alpine PipeWire tools shape image
The repository SHALL provide a Singularity definition at
`shapes/pipewire-tools/pipewire-tools.def` that builds from a minimal Alpine
base and installs the Alpine `pipewire-tools` package. The built image SHALL
include at least `pw-cat`, `pw-play`, `pw-record`, `pw-top`, `pw-profiler`,
and `pw-cli`. The image SHALL carry the standard shape labels
`com.afofo.obake.shape-schema`, `com.afofo.obake.shape-name` set to
`pipewire-tools`, and `com.afofo.obake.shape-app-version` set to the installed
PipeWire package version.

#### Scenario: Built image contains the toolset
- **WHEN** the `pipewire-tools` shape is built into a SIF
- **THEN** `pw-cat`, `pw-play`, `pw-record`, `pw-top`, `pw-profiler`, and
  `pw-cli` are present as executable commands inside the image
- **AND** the image labels identify the shape as `pipewire-tools` with an
  app version matching the Alpine PipeWire package

### Requirement: Client of the host PipeWire socket
The shape SHALL NOT start a PipeWire or WirePlumber daemon. When run as the
`obake` user with `/run/user/<uid>` bind-mounted at the same path and
`XDG_RUNTIME_DIR` set to that directory, the tools SHALL connect to the host
PipeWire socket under that runtime directory.

#### Scenario: Tools reach the host graph
- **WHEN** the host has an active PipeWire user session for `obake`
- **AND** the shape is started with the runtime-directory bind and
  `XDG_RUNTIME_DIR` set
- **THEN** `pw-cli info` (or an equivalent one-shot query) reaches the host
  remote named `pipewire-0`
- **AND** no PipeWire daemon process is started inside the container by the
  shape definition

### Requirement: Documented build and run
`shapes/README.md` SHALL list the `pipewire-tools` shape. The shape directory
SHALL document how to build the SIF and how to run a tool against the host
socket, including the required bind-mount and `XDG_RUNTIME_DIR`.

#### Scenario: Operator finds run instructions
- **WHEN** an operator opens `shapes/README.md` and the `pipewire-tools`
  shape documentation
- **THEN** they can build the image and invoke a tool such as `pw-top` or
  `pw-cat` against the host graph without reading the host recipe
