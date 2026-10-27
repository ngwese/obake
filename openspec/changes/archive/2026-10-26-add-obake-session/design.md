# Design

## Context

See proposal.md for why this exists. Constraints that shape the approach:

- `host-audio` already runs one PipeWire daemon and WirePlumber as `obake`
  user services. Linger starts that user manager at boot. Shape clients
  reach the socket only when `$XDG_RUNTIME_DIR` is bind-mounted.
- `chuck` and `rnbo-runner` already wrap their process in `pw-jack` inside
  the image. The host glibc is newer than the container's, so a host
  `libjack` bind does not load. `siren` and `mod-host` are not clients of
  this graph yet.
- `obake-ravenna` is a separate `obake` user service on `qdl`. It owns RTP
  nodes and names them with `node.name`. It reads
  `/home/obake/.config/obake-ravenna/` and reloads without restarting
  PipeWire. Other boards do not install it.
- `obake-launcher` owns `/persist/shapes` and the update health snapshot
  of that directory. It does not start containers today.
- Home is the user partition and survives OTA. The root filesystem is
  read-only. There is one Cargo crate today, `services/obake-ravenna/`,
  pinned to Rust 1.99.1, using Tokio, clap, and env_logger.
- PipeWire's `libjack` applies `PIPEWIRE_PROPS` before it fills a default
  `node.name`. The container's `pw-jack` is the library that does this.
  ChucK opens JACK under a fixed name and without `JackUseExactName`.
  Visible JACK names become `application/node` when `application.name`
  differs from `node.name`.

## Goals / Non-Goals

**Goals:**

- One control plane for the life of a rig: launch, link, record,
  restore, and expose that as GraphQL.
- A workspace so `obake-session` and `obake-ravenna` share dependency
  versions, and later a shared crate, without a behavior change to
  Ravenna.
- Milestone 1 as the first progression inside that plan, in the order in
  the proposal, with later milestones left in this change so they can be
  revised before they are built.

**Non-Goals:**

- Authentication schemes beyond a bearer token on the network listener.
  The Unix socket stays usable by the `obake` user without that token.
- Launching `siren`, `mod-host`, or `serialosc`.
- Replacing `obake-ravenna` or teaching WirePlumber to apply port links.
- Binding the host `libjack` into a shape.
- Taking ownership of `/persist/shapes` or the update health gate.
- A role that spans more than one port. Stereo is two roles.

## Decisions

### 1. Cargo workspace at services/

`services/Cargo.toml` is a workspace. Members are `obake-ravenna`,
`obake-session`, and `obake`. `services/rust-toolchain.toml` pins 1.99.1
and replaces the member file. `[workspace.dependencies]` holds the shared
versions of Tokio, clap, env_logger, `log`, serde, and `thiserror`.
`obake-session` adds `async-graphql`, `async-graphql-axum`, axum, and
`serde_yaml`.

`services/obake/` is one crate. Its library is the GraphQL client and its
binary is `obake`. Neither depends on `obake-session`, PipeWire, JACK, or
Singularity. The service crate does not depend on `obake`.
`cargo build -p obake` is the portable build.

The Ravenna image package builds from the workspace root with
`-p obake-ravenna`. The session image package builds `-p obake-session`
and, separately, `-p obake`. One `services/Cargo.lock` replaces the
member lockfile.

Alternative considered: subcommands on the `obake-session` binary.
Rejected. That executable cannot be built on a machine that does not have
the service's native libraries. Alternative considered: separate
`obake-session-client` and `obake-session-cli` crates. Rejected. One
crate named `obake` is the client and the executable.

### 2. Milestone progression

The spec is the end state. Tasks are ordered so a review can stop at a
milestone and revise the rest.

1. **Milestone 1.** Single `chuck`: rig document, one session, fresh
   and fork, start and stop, durable restore. List that rig's
   sessions, list and retrieve work-area files, and delete a session that
   is not running. Then system links from that shape's roles to Ravenna
   nodes the host already published. Then queries `rigs`,
   `rig`, and `shapes`. Then system links to the MixPre alias. Then
   several instances: ChucK into ChucK, and ChucK into `rnbo-runner`,
   linked by role. `obake` performs each of those operations through its
   client library, including over the network listener.
2. **Milestone 2.** The rest of the session contract already in the spec:
   subscriptions, the sample-library bind, and the image package if it
   was not required to demonstrate milestone 1 on hardware.
3. **Milestone 3.** Install and remove rig documents through
   GraphQL.
4. **Milestone 4.** Query the live PipeWire graph.
5. **Milestone 5.** Replace `obake-ravenna`'s configured sources and
   sinks. The session service writes that service's existing config and
   reloads it. It does not create RTP nodes itself.

Milestone 1 stores rigs as files the operator can place by hand.
Milestone 3 is the API for that same directory. Each later milestone
should be re-read against the spec before implementation; this change is
expected to be edited.

### 3. Documents on the user partition

```
/home/obake/.config/obake-session/host.yaml
/home/obake/.config/obake-session/rigs/<name>.yaml
/home/obake/.local/state/obake-session/current
/home/obake/.local/state/obake-session/sessions/<id>.yaml
/home/obake/.local/share/obake-session/rigs/<name>/work
/home/obake/.local/share/obake-session/sessions/<id>/work
/home/obake/shapes/<shape>.sif
```

`current` names the active session id, or is empty when stopped. A
mutation writes the session file to a temporary sibling, `fsync`s it,
renames it over the target, and `fsync`s the directory before the
mutation returns. Boot reads `current` and reconciles. A missing `current`
starts idle.

`/persist/shapes` stays with `obake-launcher`. This service does not write
there, so an update rollback of that directory does not roll sessions
back. Home already survives a slot switch, which is the persistence the
spec requires.

Shape images default to `/home/obake/shapes/<shape>.sif`. `host.yaml` may
set `image_dir`.

### 4. Rig document

```yaml
instances:
  lead:
    shape: chuck
    command: ["--out:2", "lead.ck"]
    roles:
      out: [melody, bed]
      in: []
links:
  - from: lead.melody
    to: space.wet
```

The instance key is the default node name. An optional `node` field
overrides it. Role lists are 1-based port slots. `from` and `to` are
`instance.role`. The file does not name `outport` or `rnbo0`.

`host.yaml` holds environment, extra binds, an optional read-only sample
library bind, `image_dir`, and card aliases:

```yaml
listen: "0.0.0.0:9477"
environment:
  OBAKE_HOST: qdl
binds: []
sample_library:
  source: /home/obake/samples
  target: /samples
aliases:
  mixpre:
    card: II
  adat:
    card: ADAT
  usbpre:
    card: USBPre2
```

An instance environment entry replaces a host entry of the same name.
Host binds and the runtime-directory bind are always applied. The runtime
directory is `$XDG_RUNTIME_DIR` of the `obake` user, mounted on the same
path, and the process environment sets `XDG_RUNTIME_DIR` to it.

### 5. Launch

`singularity instance start` runs the image. The Singularity instance
name is `obake-<session>-<instance>` so it can differ from the short node
name. Stop uses `singularity instance stop` on names this process
started.

The launch environment sets:

```
PIPEWIRE_PROPS={
  node.name=<node>,
  client.name=<node>,
  application.name=<node>,
  node.description=<node>,
  node.autoconnect=false,
  jack.self-connect-mode=ignore-external
}
```

`node.autoconnect=false` keeps WirePlumber from attaching the instance to
a default device. `jack.self-connect-mode=ignore-external` ignores
`jack_connect` from the shape itself (ChucK/RtAudio auto-wires to
physical ports otherwise). The session service is the only linker;
external `pw-link` from other clients still works.

Shape profiles live in the `obake-session` crate, not in the rig:

| Shape | Output slot n | Input slot n |
|---|---|---|
| `chuck` | `outport n` | `inport n` |
| `rnbo-runner` | `outn` | `inn` |

RNBO's live list at `/rnbo/inst/0/jack/` is checked after launch; the
client prefix is stripped before the slot is matched. A profile is the
only place that knows those strings.

Inside the container, `/work` is the rig directory and `/session` is
the session work area. Both are created before start. Fresh creates an
empty session directory and no system links. Fork copies the source
session file (new id, `forkedFrom` set) and copies the session work area.
The rig directory is not copied.

A start that fails validation does not stop the active session. A start
that passes validation stops the previous Singularity instances first,
then writes `current`, then launches.

### 6. Links

Desired links are role edges from the rig plus system edges from the
session. The reconciler polls the graph and applies a link when both
ports exist. Status is `PENDING` or `APPLIED`. Object ids are not
written. A link dropped from the desired set is disconnected.

A system endpoint is either `{ alias, port }` or `{ node, port }`.
Milestone 1 uses `{ node, port }` for Ravenna names such as
`obake-out-1` / `send_1`, which `obake-ravenna` already created. The
MixPre step resolves alias `mixpre` to the live node whose ALSA card id
is the host profile's `card` value, then uses the given port
(`playback_FL`, `capture_1`, and so on). The alias is not a PipeWire
`node.name`.

Several applied links onto one input port stay applied together.
`pipewire-jack` mixes them. The reconciler does not insert a mixer node.

Rig role links are not written into the session file. System links
are not written into the rig file.

### 7. GraphQL

Axum serves GraphQL on
`$XDG_RUNTIME_DIR/obake-session/api.sock` and on the TCP address in
`host.yaml` `listen`, default `0.0.0.0:9477`. Subscriptions use the
`graphql-ws` protocol on both. Queries and mutations are what milestone 1
needs; subscriptions land in milestone 2 on the same listeners.

```graphql
enum Direction { IN OUT }
enum SessionMode { FRESH FORK }
enum SessionPhase { PROVISIONING LINKED TEARING_DOWN }
enum LinkState { PENDING APPLIED }

type Role {
  name: String!
  direction: Direction!
  index: Int!
}

type Instance {
  name: String!
  shape: String!
  nodeName: String!
  command: [String!]!
  roles: [Role!]!
}

type RoleRef { instance: String!  role: String! }

type RigLink { from: RoleRef!  to: RoleRef! }

type Rig {
  name: String!
  instances: [Instance!]!
  links: [RigLink!]!
}

type Shape {
  name: String!
  launchable: Boolean!
  imagePresent: Boolean!
}

type SystemEndpoint {
  alias: String
  node: String
  port: String!
}

type SystemLink { from: RoleRef!  to: SystemEndpoint! }

type LinkStatus { from: String!  to: String!  state: LinkState! }

type Session {
  id: ID!
  rig: String!
  mode: SessionMode!
  forkedFrom: ID
  phase: SessionPhase!
  systemLinks: [SystemLink!]!
  linkStatus: [LinkStatus!]!
}

type SessionRecord {
  id: ID!
  rig: String!
  mode: SessionMode!
  forkedFrom: ID
}

type SessionFile {
  path: String!
  size: Int!
  directory: Boolean!
}

type Query {
  rigs: [Rig!]!
  rig(name: String!): Rig
  shapes: [Shape!]!
  session: Session
  sessions(rig: String): [SessionRecord!]!
  "Metadata only. Bytes use HTTP GET/PUT on the same listener."
  sessionFiles(id: ID!): [SessionFile!]!
  "Milestone 4. Live nodes, not the stored session."
  graph: [GraphNode!]!
}

type Mutation {
  startSession(input: StartSessionInput!): Session!
  stopSession: Boolean!
  setSystemLinks(links: [SystemLinkInput!]!): Session!
  deleteSession(id: ID!): Boolean!
  "Milestone 3."
  installRig(name: String!, document: String!): Rig!
  "Milestone 3."
  removeRig(name: String!): Boolean!
  "Milestone 5. Writes obake-ravenna config and reloads it."
  setRavennaStreams(
    sources: [RavennaSourceInput!]!
    sinks: [RavennaSinkInput!]!
  ): Boolean!
}

type Subscription {
  session: Session
  links: [LinkStatus!]!
}

input StartSessionInput {
  rig: String!
  mode: SessionMode!
  forkFrom: ID
}

input RoleRefInput { instance: String!  role: String! }

input SystemEndpointInput {
  alias: String
  node: String
  port: String!
}

input SystemLinkInput {
  from: RoleRefInput!
  to: SystemEndpointInput!
}

type GraphPort { name: String!  direction: Direction! }
type GraphNode { name: String!  ports: [GraphPort!]! }

"Milestone 5. Same fields obake-ravenna already accepts."
input RavennaSourceInput {
  name: String!
  channels: Int!
  format: String!
}
input RavennaSinkInput {
  name: String!
  device: String!
  stream: String!
}
```

`installRig`'s `document` is the rig YAML. That keeps milestone
3 from inventing a second document model. `setRavennaStreams` replaces
the source and sink arrays in the existing Ravenna config and reloads
that user service. It is unimplemented on boards that do not ship
`obake-ravenna`, and the mutation returns an error there. Fields beyond
name, channels, format, device, and stream stay as the Ravenna file left
them until a later edit of this change adds them.

`session` is null when idle. Phase is `PROVISIONING` while any instance
is starting or any desired link is pending, `LINKED` when every desired
link is applied (including when there are no links) and every instance is
running, and `TEARING_DOWN` during stop.

`shapes` lists the profiles compiled into the service. `launchable` is
true only for `chuck` and `rnbo-runner` with a SIF in `image_dir`.
`siren` and `mod-host` are not profiles in this change, so they are
absent even if a SIF exists.

`sessions(rig:)` omits the argument to list every recorded session.
`sessionFiles` lists metadata in the session work area only. File bytes
are not carried through GraphQL. `..` and any absolute path fail before
a file is opened. `deleteSession` removes that session's snapshot and
work area and does not follow `forkedFrom`. It fails while that session
is the one named by `current`.

#### HTTP(S) session files

The same Unix socket and TCP listener that serve GraphQL also serve
raw work-area bytes:

```
GET  /sessions/{id}/files/{*path}
PUT  /sessions/{id}/files/{*path}
GET  /sessions/current/files/{*path}
PUT  /sessions/current/files/{*path}
```

`current` means the active session named by `current`. When idle, those
routes fail. The body is the file octets; there is no base64 wrapper.
`Content-Type` is `application/octet-stream`. Path rules match
`sessionFiles`: relative to that session's work area only. TCP uses the
same bearer token as GraphQL; the Unix socket does not. PUT creates or
replaces a regular file and fails on a path that would escape the work
area or overwrite a directory.

#### `obake` CLI shape

The service binary is `obake-session run` only. `obake` is the operator
tool and the library. Control-plane calls post GraphQL; `file get` and
`file put` use HTTP(S) on the same resolved endpoint. Resolution order
is `--endpoint` and `--token`, then the host selected by `--host`, then
the config file's `default` host, then the user Unix socket. The file
is `$XDG_CONFIG_HOME/obake/config.yaml`, or
`~/.config/obake/config.yaml` when `XDG_CONFIG_HOME` is unset. `--config`
selects another path. A missing file is not an error. An unknown
`--host`, or a `default` that is not in `hosts`, fails before a request.

```yaml
default: studio
hosts:
  studio:
    endpoint: http://192.168.60.10:9477
    token: <bearer>
  spare:
    endpoint: http://192.168.60.11:9477
    token: <bearer>
```

That file lives on the machine running `obake`. It is not the host's
`/home/obake/.config/obake-session/token`. The library loads the same
file for other programs.

By default every subcommand is `<noun> <verb>`. Nouns are schema
entities (`rig`, `shape`, `session`, `link`, `file`, `graph`, …).
Verbs are `ls`, `start`, `stop`, `create`, `delete`, `get`, and
`describe`. A plural noun alone is not a command: list with
`<noun> ls` (for example `rig ls`, not `rigs`). Exceptions are allowed
when a flat verb would be unclear; this change keeps `ravenna set` and
`watch` as exceptions.

| Command | Role |
|---|---|
| `rig ls` | list installed rigs |
| `rig describe <name>` | one rig |
| `rig create <name> --file …` | install a rig document |
| `rig delete <name>` | remove a rig |
| `shape ls` | list shapes |
| `session ls [--rig]` | list recorded sessions |
| `session describe` | active session (fails when idle) |
| `session describe <id>` | one recorded session |
| `session start <rig>` | start (fresh/fork flags as today) |
| `session stop` | stop the active session |
| `session delete <id>` | delete an inactive session |
| `link set …` | replace system links |
| `file ls [--session <id>]` | list work-area files (active if omitted) |
| `file get <path> <dest> [--session <id>]` | HTTP GET bytes (like `cp`) |
| `file put <local> <dest> [--session <id>]` | HTTP PUT bytes |
| `graph ls` | live PipeWire graph |
| `ravenna set` | exception: replace Ravenna streams |
| `watch` | exception: subscriptions |

`file get` destinations: `.` writes a local file with the remote
basename; `-` streams to stdout; any other path is the local target.
`--session` on `file ls`, `file get`, and `file put` selects a recorded
session; omitting it uses the active session.

The bearer token is `/home/obake/.config/obake-session/token`, mode
`0600`, created when missing and left in place when present. TCP
requests must send it as `Authorization: Bearer`. The Unix socket does
not check it. The socket directory is reachable by the `obake` user.

### 8. Image

`image/os/buildroot/package/obake-session/` builds the workspace members.
The service build is `-p obake-session` and links the host audio stack.
The `obake` build is a separate `-p obake` and does not.
`image/os/buildroot/package/obake-runtime/Config.in` selects the package,
so every board with the runtime layer gets both binaries and the user
unit. `obake-ravenna` stays selected only by the `qdl` config.

The user unit is `After=pipewire.service` and `WantedBy=default.target`.
It is not a system unit. A tmpfiles snippet creates the home config and
state directories, owned by `obake`, only when missing, with no rig
files.

The Ravenna package's cargo invocation changes to the workspace root and
`-p obake-ravenna`. Its installed binary, unit, and config seed stay as
they are.

## Risks / Trade-offs

- [ChucK ignores a renamed client] → Ports are registered on the JACK
  handle, and the graph name comes from `node.name`. A milestone 1 test
  must show `pw-link` listing `lead:outport 1` for an instance named
  `lead`. If the container's `pipewire-jack` is too old to honor
  `PIPEWIRE_PROPS` before the default name, the profile gains a small
  `jack_client_open` preload rather than a ChucK rebuild.
- [RNBO keeps `rnbo0`] → The profile matches the port suffix from
  OSCQuery. The node name still has to be the instance name. Two RNBO
  instances in one process are out of milestone 1; one `rnbo-runner`
  instance maps to `/rnbo/inst/0`.
- [Two writers of Ravenna config] → Until milestone 5, this service only
  links to nodes Ravenna already created. Milestone 5 writes the file and
  reloads; it does not edit the file while also keeping a second copy.
- [Home is not in the update snapshot] → A bad session survives a slot
  rollback. Accepted for this change. Connecting `obake-launcher` to
  session restore is a later edit, not a hidden task.
- [Ext4 copies] → Fork copies the session work area file by file. Large
  takes are slow. No snapshot filesystem is assumed.
- [Later milestones drift] → Milestones 3–5 are in the spec so the plan
  has a contract, and in the tasks so the order is visible. Revise those
  sections before implementing them. Each of those tasks adds the matching
  `obake` subcommand in the same change.
- [Network listener] → The default bind is every interface. The bearer
  token is the gate. A stolen token can start and delete sessions. The
  token file stays in the home directory and is not part of a rig.

## Migration Plan

1. Land the workspace and keep `cargo test` green for `obake-ravenna`
   before `obake-session` launches anything.
2. Ship the user unit with an empty home seed. Hosts that already have
   the config directories are left alone.
3. Rollback is the previous OS slot. Home state remains. The previous
   slot does not run `obake-session`, so rigs stay on disk unused.
   Ravenna's config format is untouched until milestone 5.
