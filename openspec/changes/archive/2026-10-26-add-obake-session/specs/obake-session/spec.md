# Spec Delta

## Purpose

Runs one rig of shapes on the host graph, restores that session after
a restart, and exposes the control plane as GraphQL.

## ADDED Requirements

### Requirement: Installed rigs are readable

The service SHALL list every rig definition installed for the `obake`
user, and SHALL return one rig by name. Each entry SHALL include its
instances, each instance's shape, node name, command arguments, and ordered
roles, and the rig's role links.

#### Scenario: A query lists an installed rig

- **WHEN** a rig named `quartet` is installed and a client queries
  the rig list
- **THEN** the list includes `quartet` with its instances, roles, and role
  links

#### Scenario: An unknown name returns nothing

- **WHEN** a client queries a rig name that is not installed
- **THEN** the query returns no rig

### Requirement: An instance name is its node name

Each instance key SHALL be unique in its rig and SHALL be the
PipeWire node name of that process. The key SHALL NOT contain a colon or
a slash. Two instances of the same shape SHALL still receive two node
names.

#### Scenario: Two ChucK instances stay distinct

- **WHEN** a rig starts instances `lead` and `harmony`, both of
  shape `chuck`
- **THEN** the graph contains nodes named `lead` and `harmony`

#### Scenario: A duplicate or unsafe name is rejected

- **WHEN** a rig repeats an instance key, or a key contains `:` or
  `/`
- **THEN** the rig cannot be started

### Requirement: A role names one port

Output roles and input roles SHALL be ordered lists. The role at position
n, counting from 1, SHALL be that instance's nth port in that direction.
A rig link SHALL name `instance.role` on each end and SHALL NOT name
a shape or a shape-specific port string.

#### Scenario: Role order selects the port

- **WHEN** instance `lead` has output roles `melody` then `bed`
- **THEN** `lead.melody` is `lead`'s first output port and `lead.bed` is
  its second output port

#### Scenario: The stored link uses roles

- **WHEN** a rig links `lead.melody` to `space.wet`
- **THEN** the stored link records those two role addresses and does not
  record a shape name or a software port name

### Requirement: Only complete JACK shapes launch

The service SHALL start an instance whose shape is `chuck` or
`rnbo-runner`. It SHALL reject a start whose rig names any other
shape, and that rejection SHALL leave the active session unchanged.

#### Scenario: ChucK and RNBO start

- **WHEN** a client starts a session for a rig whose instances are
  only `chuck` and `rnbo-runner`
- **THEN** each of those instances is running

#### Scenario: An incomplete shape does not replace the session

- **WHEN** a session is active and a client starts a rig that
  contains shape `siren` or `mod-host`
- **THEN** the start fails and the previously active session is still
  running

### Requirement: At most one session is active

The service SHALL run at most one session. A successful start while
another session is active SHALL stop that session's instances before
starting the new ones. A start that fails validation SHALL NOT stop the
active session.

#### Scenario: A successful start replaces the active session

- **WHEN** a session for rig `quartet` is active and a client
  successfully starts rig `duo`
- **THEN** `quartet`'s instances are stopped and only `duo`'s instances
  are running

### Requirement: A fresh session starts an empty take

A fresh session SHALL create the rig working directory if it is
missing, SHALL reuse that directory, and SHALL start an empty session
work area with no system links.

#### Scenario: Fresh reuses the piece and starts an empty take

- **WHEN** a client starts rig `quartet` in fresh mode and that
  rig's working directory already holds `lead.ck`
- **THEN** the new session has a new id, an empty system-link list, and
  an empty session work area
- **AND** `lead.ck` is still the file in the rig working directory

### Requirement: A fork copies the take and shares the piece

A fork SHALL allocate a new session id, SHALL copy the source session's
system links and session work area, and SHALL keep the same rig
working directory rather than copying it.

#### Scenario: Fork copies the take and shares the piece

- **WHEN** a client forks an existing session whose work area holds
  `take.wav` and whose rig directory holds `lead.ck`
- **THEN** the new session has a new id, the same system links, and its
  own copy of `take.wav`
- **AND** the rig working directory is still the one directory that
  holds `lead.ck`

### Requirement: Instances see both work areas

Every running instance SHALL see the rig working directory at
`/work` and the session work area at `/session`.

#### Scenario: Both directories are mounted

- **WHEN** a session for rig `quartet` is running and the rig
  directory holds `lead.ck`
- **THEN** each instance sees that file at `/work/lead.ck` and sees the
  session work area at `/session`

### Requirement: The recorded session survives restart

The service SHALL record the active session on the user partition before
acknowledging a mutation that changes it. After a process restart or a
host boot, including a power loss, the service SHALL start that session
again. A stopped session SHALL stay stopped across boot and SHALL NOT
launch a rig.

#### Scenario: Power loss restores the active session

- **WHEN** a session is active and the host loses power without a
  shutdown
- **THEN** the next boot starts the same rig and reapplies its
  recorded links

#### Scenario: Stop stays stopped

- **WHEN** a client stops the active session and the host reboots
- **THEN** no rig is started

### Requirement: Instances join the host PipeWire graph

Every instance SHALL be launched with the `obake` PipeWire runtime
directory visible and `XDG_RUNTIME_DIR` set to it. The service SHALL NOT
mount the host `libjack` into the instance.

#### Scenario: A shape opens the host socket

- **WHEN** a `chuck` instance starts
- **THEN** it can open the `obake` user's PipeWire socket
- **AND** the host `libjack` is not mounted in the container

### Requirement: The host profile is layered under every instance

Environment and binds from the host profile SHALL be applied to every
instance. A rig value of the same environment name SHALL replace
the host value. A sample-library bind in the host profile SHALL be
mounted read-only into every instance.

#### Scenario: Rig environment replaces the host value

- **WHEN** the host profile sets `OBAKE_HOST` and the instance also sets
  `OBAKE_HOST`
- **THEN** the process receives the instance's value

#### Scenario: The sample library is read-only

- **WHEN** the host profile binds a sample library into every instance
- **THEN** each instance sees that directory, and a write inside the
  instance fails

### Requirement: Links are applied when their ports exist

The service SHALL apply every rig role link and every session system
link when both ports exist, and SHALL report any other desired link as
pending. Several links onto one input role SHALL all be applied, and that
input SHALL mix them. The service SHALL remove a link that is no longer
desired. PipeWire object ids SHALL NOT be stored.

#### Scenario: A link waits for a port

- **WHEN** a desired role link names a port that does not exist yet
- **THEN** that link is reported pending and is not left half-connected

#### Scenario: Two sources mix at one input

- **WHEN** rig links `lead.melody` and `harmony.pad` both target
  `space.wet` and all three ports exist
- **THEN** both links are applied to `space.wet`

#### Scenario: A removed link is disconnected

- **WHEN** a client removes a system link that was applied
- **THEN** those ports are no longer connected and the session no longer
  records the link

### Requirement: System links belong to the session

A system link SHALL connect an instance role to either a host card alias
and port, or a node name and port that this service does not own. The
service SHALL resolve a card alias through the host profile while the
card is present. System links SHALL be stored on the session and SHALL
NOT be written into the rig definition.

#### Scenario: A card alias links while the card is present

- **WHEN** the host profile aliases `mixpre` to the MixPre card and a
  system link targets `mixpre` port `playback_FL`
- **THEN** the role is connected to that card's `playback_FL` while the
  card is present, and the link is pending while the card is absent
- **AND** the rig definition is unchanged

#### Scenario: A Ravenna node is a session target

- **WHEN** a system link targets node `obake-out-1` port `send_1` and
  that port exists
- **THEN** the role is connected to `obake-out-1:send_1`
- **AND** the rig definition does not contain that node name

### Requirement: Known shapes are queryable

The service SHALL serve a GraphQL query `shapes`. Each entry SHALL name
a shape the service knows, whether that shape is launchable, and whether
its image is present. `chuck` and `rnbo-runner` SHALL be launchable when
their images are present. `siren` and `mod-host` SHALL NOT be launchable.

#### Scenario: ChucK is listed when its image is present

- **WHEN** a client queries `shapes` and the `chuck` image is present
- **THEN** the result includes `chuck` as launchable and present

#### Scenario: An incomplete shape is not launchable

- **WHEN** a `siren` or `mod-host` image is present and a client queries
  `shapes`
- **THEN** that shape is either absent or marked not launchable

### Requirement: GraphQL reads rigs and the session

The service SHALL serve GraphQL queries `rigs`, `rig(name)`,
`session`, and `sessions`. `session` SHALL be the active session or empty.
`sessions` SHALL list recorded sessions with id, rig name, mode, and
fork source, and SHALL accept a rig name that limits the list to
that rig. Session mode SHALL be `FRESH` or `FORK`.

#### Scenario: The active session is readable

- **WHEN** a client queries `session` while a session is active
- **THEN** the result includes that session's id, rig name, mode,
  phase, system links, and per-link status

#### Scenario: Idle has no active session

- **WHEN** a client queries `session` after a stop
- **THEN** the result is empty

#### Scenario: A rig lists only its sessions

- **WHEN** two recorded sessions are for rig `quartet` and one is
  for rig `duo`
- **THEN** `sessions` for `quartet` returns those two and not the `duo`
  session

### Requirement: GraphQL starts, stops, and records system links

The service SHALL serve mutations `startSession`, `stopSession`, and
`setSystemLinks`. `startSession` SHALL accept a rig name and a mode.
Fork mode SHALL require the id of an existing session. `setSystemLinks`
SHALL replace the active session's system links with the given list.

#### Scenario: Fork requires a source id

- **WHEN** a client calls `startSession` with mode `FORK` and no source id
- **THEN** the mutation fails and no new session is created

#### Scenario: setSystemLinks replaces the list

- **WHEN** a client calls `setSystemLinks` with one link
- **THEN** the active session records that link and no previous system
  link

### Requirement: GraphQL subscriptions follow the session

The service SHALL serve subscriptions `session` and `links`. A change to
the active session SHALL be delivered on `session`. A change to any
desired link's pending or applied status SHALL be delivered on `links`.

#### Scenario: A subscriber sees a start

- **WHEN** a client is subscribed to `session` and another client starts
  a session
- **THEN** the subscriber receives the new active session

#### Scenario: A subscriber sees a link become applied

- **WHEN** a client is subscribed to `links` and a pending link's ports
  both appear
- **THEN** the subscriber receives that link as applied

### Requirement: Rigs can be installed and removed

The service SHALL serve mutations that install a rig document and
remove a rig by name. A removed rig SHALL no longer appear
in the rig list. Removing the rig of the active session SHALL
fail and SHALL
leave that session running.

#### Scenario: An installed rig becomes startable

- **WHEN** a client installs a document named `duo` whose instances are
  `chuck` and `rnbo-runner`
- **THEN** `rigs` includes `duo` and `startSession` can select it

#### Scenario: Removing the active rig fails

- **WHEN** a session for `duo` is active and a client removes `duo`
- **THEN** the removal fails, `duo` is still installed, and the session
  is still running

### Requirement: The live graph is queryable

The service SHALL serve a GraphQL query of the PipeWire nodes and ports
it can currently see. The result SHALL be the live graph and SHALL NOT
be a copy of the stored session.

#### Scenario: A running instance appears in the graph query

- **WHEN** instance `lead` is running and a client queries the live graph
- **THEN** the result includes node `lead` and the ports that node has
  now

### Requirement: Ravenna streams can be replaced

On a host where `obake-ravenna` is installed, the service SHALL serve a
mutation that replaces that service's configured sources and sinks with
the submitted lists. The mutation SHALL NOT restart PipeWire. Where
`obake-ravenna` is not installed, the mutation SHALL fail and SHALL NOT
create a Ravenna configuration.

#### Scenario: Submitted streams become the Ravenna configuration

- **WHEN** a client submits one source and one sink and `obake-ravenna`
  is installed
- **THEN** that service's configured sources and sinks are those two
- **AND** PipeWire was not restarted

#### Scenario: A host without Ravenna rejects the mutation

- **WHEN** a client submits streams and `obake-ravenna` is not installed
- **THEN** the mutation fails and no Ravenna configuration is created

### Requirement: Session work-area files can be listed and transferred

The service SHALL list the files in a session's work area through
GraphQL. It SHALL serve HTTP GET and HTTP PUT of raw file bytes for a
path relative to that work area on the same listeners as GraphQL. A path
that escapes the work area, or a GET that names a missing file, SHALL
fail. PUT SHALL create or replace a regular file and SHALL NOT overwrite
a directory. GraphQL SHALL NOT be the path that carries file content.

#### Scenario: A file in the work area is returned over HTTP

- **WHEN** a session work area contains `take.wav` and a client GETs
  that path for that session
- **THEN** the response body is the bytes of `take.wav` with no base64
  encoding

#### Scenario: A file can be written over HTTP

- **WHEN** a client PUTs bytes to `scratch.ck` in a session work area
- **THEN** that path holds those bytes and a later list includes it

#### Scenario: A path outside the work area fails

- **WHEN** a client GETs or PUTs `../lead.ck`
- **THEN** the request fails and no file outside the work area is read
  or written

### Requirement: A session can be deleted

`deleteSession` SHALL remove that session's record and work area. It
SHALL fail when the session is active, and the active session SHALL keep
running. It SHALL NOT remove the rig working directory or any other
session.

#### Scenario: An inactive session and its files are removed

- **WHEN** a client deletes a session that is not active and whose work
  area holds `take.wav`
- **THEN** that session is no longer listed and `take.wav` is gone
- **AND** the rig working directory is unchanged

#### Scenario: The active session cannot be deleted

- **WHEN** a client deletes the active session
- **THEN** the deletion fails and that session is still active

### Requirement: A separate CLI performs every API operation

`obake` SHALL be an executable other than the service, and the client
library other programs link. It SHALL perform control-plane queries and
mutations through GraphQL, and SHALL transfer session file bytes through
HTTP(S). Building it SHALL NOT require PipeWire, JACK, or Singularity,
and SHALL NOT require building the service.

#### Scenario: The CLI starts a session through GraphQL

- **WHEN** the operator runs `obake session start` for a rig
- **THEN** the service receives a `startSession` operation and that
  session becomes active

#### Scenario: The CLI does not need the host audio stack

- **WHEN** `obake` is built
- **THEN** that build does not require PipeWire, JACK, Singularity, or
  the service executable

### Requirement: The CLI uses noun-verb commands

`obake` subcommands SHALL follow `<noun> <verb>` by default. Nouns SHALL
be schema entities. Verbs SHALL include `ls`, `start`, `stop`, `create`,
`delete`, `get`, and `describe`. A plural noun SHALL NOT be a top-level
command; listing SHALL use `<noun> ls`. Documented exceptions MAY omit
the pattern when a flat verb would be unclear.

#### Scenario: Listing uses noun ls

- **WHEN** the operator lists installed rigs
- **THEN** the command is `obake rig ls` and not a plural top-level
  `rigs` command

#### Scenario: Session describe defaults to the active session

- **WHEN** a session is active and the operator runs
  `obake session describe` with no id
- **THEN** the output describes that active session

#### Scenario: File commands default to the active session

- **WHEN** a session is active and the operator runs `obake file ls`
  without `--session`
- **THEN** the listed files are that session's work area

#### Scenario: File get writes like cp

- **WHEN** the operator runs `obake file get take.wav .` for the active
  session
- **THEN** the client performs an HTTP GET of the raw bytes and writes a
  local file named `take.wav`

#### Scenario: File get can stream to stdout

- **WHEN** the operator runs `obake file get take.wav -`
- **THEN** the raw bytes are written to stdout

#### Scenario: File put uploads bytes

- **WHEN** the operator runs `obake file put ./local.ck remote.ck`
- **THEN** the client performs an HTTP PUT of the local file bytes into
  the session work area at `remote.ck`

#### Scenario: File commands accept --session

- **WHEN** the operator passes `--session <id>` to `file ls`, `file get`,
  or `file put`
- **THEN** the operation targets that session's work area instead of the
  active session

### Requirement: The client config names hosts and a default

The `obake` client config SHALL list hosts, each with an endpoint and a
bearer token, and SHALL name the default host. The client SHALL use that
default unless a command names another host. A command-line endpoint or
token SHALL replace the chosen host's value. A missing file SHALL leave
the local Unix socket in use. An unknown host name SHALL fail and SHALL
NOT send a request.

#### Scenario: The default host is used

- **WHEN** the config names hosts `studio` and `spare`, sets the default
  to `studio`, and the operator runs `obake` without naming a host
- **THEN** the request goes to `studio`'s endpoint with `studio`'s token

#### Scenario: A command selects another host

- **WHEN** that config's default is `studio` and the operator names
  `spare`
- **THEN** the request goes to `spare`'s endpoint with `spare`'s token

#### Scenario: A flag replaces the chosen host

- **WHEN** the chosen host has an endpoint and the operator passes a
  different `--endpoint`
- **THEN** the request goes to the flag's endpoint

#### Scenario: A missing file uses the local socket

- **WHEN** no config file exists and the operator passes no endpoint
- **THEN** the client uses the local Unix socket

#### Scenario: An unknown host is rejected

- **WHEN** the operator names a host that is not in the config
- **THEN** the command fails and no request is sent

### Requirement: GraphQL and file HTTP are served locally and on the network

The service SHALL accept GraphQL and session-file HTTP on a Unix socket
in the `obake` user's runtime directory and on a configured TCP address.
A request to the TCP address SHALL include the host's bearer token. A
TCP request without that token SHALL be rejected and SHALL NOT change
the session or work-area files. A request on the Unix socket SHALL NOT
require the token.

#### Scenario: A tokened network client is served

- **WHEN** a client queries the configured TCP address with the bearer
  token
- **THEN** the query result is returned

#### Scenario: A network request without the token is rejected

- **WHEN** a client queries the configured TCP address without the
  bearer token
- **THEN** the request is rejected and the active session is unchanged

#### Scenario: The local socket does not require the token

- **WHEN** a client queries the user Unix socket without a bearer token
- **THEN** the query result is returned

#### Scenario: Network file GET requires the token

- **WHEN** a client GETs a session file on the TCP address without the
  bearer token
- **THEN** the request is rejected and no file bytes are returned
