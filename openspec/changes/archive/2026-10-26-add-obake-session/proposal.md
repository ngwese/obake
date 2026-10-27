# Proposal

## Why

Shapes are started by hand, and the links between them disappear when the
graph or the host restarts. The headless server needs one resident service
that brings a named rig up and down, restores that choice after a
power loss, and lets local and remote clients make the change.

## What Changes

- An `obake` user service, `obake-session`, lists the rigs installed
  on the host and runs at most one of them. Boot restores the last
  recorded session, including after a power cut. A stopped session stays
  stopped.
- A rig names shape instances, gives each instance a graph node
  name, assigns roles to its ports, and records links between those roles.
  The first shapes the service can launch are `chuck` and `rnbo-runner`.
- Each instance receives a merged launch: the rig's own command and
  binds, plus host-wide binds and environment (PipeWire runtime directory,
  shared metadata, an optional sample library). The instance name is the
  PipeWire node name.
- Links inside a rig are part of the piece and use role names. Links
  from a role to a host audio endpoint (a card alias, or a Ravenna node)
  are part of the session and stay on this host.
- A fresh session reuses the rig's working directory and starts an
  empty session work area. A fork copies the session snapshot and the
  session work area, and leaves the rig working directory shared.
  The sessions of one rig can be listed. Each session's work-area
  files can be listed and transferred as raw bytes. A session that is
  not running can be deleted, which removes its record and its work
  area.
- Clients speak GraphQL (query, mutation, subscription) on a Unix socket
  in the `obake` runtime directory and on a configured network address,
  and speak HTTP(S) on the same listeners for session work-area file
  bytes. A network request must present the host's bearer token. The
  GraphQL schema covers rigs, the shapes the host can launch, session
  start and stop, system links, and link status. Later in the same plan
  it also installs and removes
  rig documents, reads the live PipeWire graph, and replaces the
  Ravenna sources and sinks `obake-ravenna` already reconciles.
- Operators use a separate executable, `obake`, which is also the client
  library other programs link. It compiles without the service and
  without PipeWire, JACK, or Singularity, so it can be built on another
  machine and pointed at the host's network listener. The client config
  names several hosts and which one is the default. A flag selects
  another host or replaces that host's endpoint or token. Subcommands
  follow a `<noun> <verb>` pattern by default (plurals become
  `<noun> ls`). GraphQL covers control-plane operations; file get and
  put use HTTP(S). The service itself stays on the host.
- The work is ordered. Milestone 1 is the first progression and is the
  part expected to move first while the rest of the plan is iterated:
  start and stop a single `chuck` with a rig, a session, and
  session forking; list that rig's sessions and their files, and
  delete a session; wire that shape to system-supplied Ravenna sources
  and sinks; query rigs and shapes; wire that shape to
  system-supplied MixPre sources and sinks; then start and stop several
  shapes, including ChucK into ChucK and ChucK into RNBO. The CLI
  covers the operations that exist in that milestone.

## Capabilities

### New Capabilities

- `obake-session`: rig definitions, single-session lifecycle, role
  links, durable restore, the GraphQL control API, and HTTP(S) session
  file transfer, including the later
  rig, graph, and Ravenna edits.

### Modified Capabilities

- `os-image-build`: the runtime layer installs `obake-session` as an
  enabled `obake` user service on every board. `obake-ravenna` stays
  limited to `qdl`.

## Impact

- Persistent state lives on the user partition, under the `obake` home
  directory, so OTA slot switches leave rigs, session snapshots, and
  working directories in place. The root filesystem stays read-only.
- A rig file is the portable piece (instances, roles, role links,
  rig working directory). System links and card aliases are
  host-specific and are not part of that piece.
- `services/` becomes one Cargo workspace. `obake-ravenna` keeps its
  behavior and moves to a workspace member; its image package builds that
  member from the shared lockfile. The session service depends on Tokio,
  async-graphql, axum, clap, and env_logger. The `obake` crate does not
  depend on the service crate.
- `chuck` and `rnbo-runner` images are unchanged. `siren` and `mod-host`
  are not launch targets. The service does not bind the host `libjack`
  into a shape.
- GraphQL and HTTP(S) file routes are served on the user-runtime Unix
  socket and on a configured TCP port. The network listener requires a
  bearer token kept in the `obake` home directory. An image update does
  not replace an existing token.
