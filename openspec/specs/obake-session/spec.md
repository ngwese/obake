# obake-session Specification

## Purpose

Runs one rig of shapes on the host graph, restores that session after
a restart, and exposes the control plane as GraphQL.

## Requirements

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

A fresh session SHALL allocate a new session id and a new session work
directory mounted for instances at `/work`. When the selected rig is a
directory package with a `work/` template, that template SHALL be copied
into the new session work directory. When the rig is a single file, or
the package has no `work/` template, the session work directory SHALL
start empty. A fresh session SHALL NOT reuse another session's work
directory and SHALL NOT leave a shared writable piece for that rig.
System links SHALL start empty.

#### Scenario: Fresh reuses the piece and starts an empty take

- **WHEN** a client starts directory package `tone` in fresh mode and
  that package's `work/` holds `tone.ck`
- **THEN** the new session has a new id, an empty system-link list, and
  a session work directory that contains a copy of `tone.ck`
- **AND** a later edit to that session's `tone.ck` does not change the
  package template

#### Scenario: Fresh with a single-file rig starts empty

- **WHEN** a client starts single-file rig `duo` in fresh mode
- **THEN** the new session's work directory is empty

#### Scenario: A second fresh start does not inherit the prior take

- **WHEN** a session for `tone` has a modified `tone.ck` and a client
  starts `tone` again in fresh mode
- **THEN** the new session's `tone.ck` matches the package template,
  not the prior session's edit

### Requirement: A fork copies the take and shares the piece

A fork SHALL allocate a new session id, SHALL copy the source session's
system links and session work directory, and SHALL NOT read or write the
rig package template as part of the copy.

#### Scenario: Fork copies the take and shares the piece

- **WHEN** a client forks an existing session whose work directory holds
  a modified `tone.ck`
- **THEN** the new session has a new id, the same system links, and its
  own copy of that modified `tone.ck`
- **AND** the package template is unchanged

### Requirement: Instances see both work areas

Every running instance SHALL see that session's work directory at
`/work`. The service SHALL NOT mount a second writable session area and
SHALL NOT mount the rig package template into the instance.

#### Scenario: Both directories are mounted

- **WHEN** a session for rig `tone` is running and that session's work
  directory holds `tone.ck`
- **THEN** each instance sees that file at `/work/tone.ck`
- **AND** no `/session` bind is present

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

### Requirement: A rig package is a file or a directory

An installed rig SHALL be either a single YAML file
`rigs/<name>.yaml` or a directory `rigs/<name>/` whose definition is
`rig.yaml`. A directory package MAY include a `work/` tree that is the
template for fresh session work. The package SHALL live under the
service config tree. Session work directories SHALL live under the share
tree and SHALL NOT be the package template. Listing and describe SHALL
treat both forms as the same rig name. A `shapes/` directory inside a
package is reserved and SHALL NOT be required in this change.

#### Scenario: A single-file package is listed by stem name

- **WHEN** `rigs/duo.yaml` is installed
- **THEN** the rig list includes `duo`

#### Scenario: A directory package is listed by directory name

- **WHEN** `rigs/tone/rig.yaml` is installed
- **THEN** the rig list includes `tone`

#### Scenario: File and directory forms of the same name conflict

- **WHEN** both `rigs/tone.yaml` and `rigs/tone/` would exist
- **THEN** install or load fails and neither form is silently preferred

### Requirement: Rigs can be installed and removed

The service SHALL install a rig as either a single YAML document or a
directory package under the config rigs tree, and SHALL remove a rig by
name. A removed package SHALL no longer appear in the rig list. Removing
the rig of the active session SHALL fail and SHALL leave that session
running. Installing under a name that already exists SHALL replace that
package. The service SHALL reject a layout that would place both
`<name>.yaml` and `<name>/` for the same name.

#### Scenario: An installed rig becomes startable

- **WHEN** a client installs a document named `duo` whose instances are
  `chuck` and `rnbo-runner`
- **THEN** `rigs` includes `duo` and `startSession` can select it

#### Scenario: An installed directory package becomes startable

- **WHEN** a client installs directory package `tone` that contains
  `rig.yaml` and `work/tone.ck`
- **THEN** `rigs` includes `tone` and a fresh start seeds `tone.ck`

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

`deleteSession` SHALL remove that session's record and work directory.
It SHALL fail when the session is active, and the active session SHALL
keep running. It SHALL NOT remove the rig package under the config tree
or any other session.

#### Scenario: An inactive session and its files are removed

- **WHEN** a client deletes a session that is not active and whose work
  directory holds `take.wav`
- **THEN** that session is no longer listed and `take.wav` is gone
- **AND** the installed rig package is unchanged

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
`delete`, `get`, `describe`, and `init`. A plural noun SHALL NOT be a
top-level command; listing SHALL use `<noun> ls`. Documented exceptions
MAY omit the pattern when a flat verb would be unclear. The nouns
`auth` and `device` are documented exceptions and are not schema
entities.

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

#### Scenario: Auth is an allowed noun

- **WHEN** the operator enrolls, approves a browser, or rotates the
  bearer
- **THEN** the commands are `obake auth login`, `obake auth approve`,
  and `obake auth rotate`

#### Scenario: Device is an allowed noun

- **WHEN** the operator updates the host OS through the CLI
- **THEN** the command is `obake device update`

#### Scenario: Rig init scaffolds a local package

- **WHEN** the operator runs `obake rig init tone`
- **THEN** a local directory `tone/` is created with `rig.yaml` and an
  optional `work/` tree, without contacting the session service

### Requirement: Device update pushes a bundle over SSH

`obake device update` SHALL require `--target <ssh-user@host>` and a
local `PATH` that is either a `.raucb` file or a directory containing
such files. It SHALL use the system SSH client to read the device's
running image version and active boot slot, select or validate a bundle,
copy that bundle to the device, and run
`obake-update apply --cleanup --reboot` on the staged path. It SHALL NOT
use the session GraphQL or file HTTP APIs. A missing or unreachable SSH
target SHALL fail without modifying the device.

#### Scenario: Update requires an SSH target

- **WHEN** the operator runs `obake device update ./bundles` without
  `--target`
- **THEN** the command fails before contacting any host

#### Scenario: Update applies over SSH when the session is down

- **WHEN** the operator runs
  `obake device update --target obake@studio.local ./bundles` and SSH
  reaches the host while `obake-session` is not running
- **THEN** the selected bundle is installed through `obake-update` and
  the host is asked to reboot

#### Scenario: Unreachable target fails closed

- **WHEN** the operator runs
  `obake device update --target obake@missing.local ./bundles` and SSH
  cannot connect
- **THEN** the command fails and no bundle is copied

### Requirement: Device update selects a directory bundle by slot and semver

When `PATH` is a directory, `obake device update` SHALL consider only
files named `obake-<semver>-<slot>.raucb` whose `<slot>` is the device's
inactive boot slot (`a` when the active slot is B, `b` when it is A).
Among those files whose `<semver>` is greater than or equal to the
device's running image version, it SHALL select the greatest semver. When
no such file exists, the command SHALL fail without copying a bundle.
`--force` SHALL NOT change directory selection.

#### Scenario: Directory picks the newest eligible inactive-slot bundle

- **WHEN** the device runs version `0.7.0` on slot B and the directory
  holds `obake-0.7.0-a.raucb`, `obake-0.8.0-a.raucb`, and
  `obake-0.8.0-b.raucb`
- **THEN** the command selects `obake-0.8.0-a.raucb`

#### Scenario: Directory rejects when nothing is new enough

- **WHEN** the device runs version `0.8.0` and the directory only holds
  `obake-0.7.0-a.raucb` for the inactive slot
- **THEN** the command fails without copying a bundle

#### Scenario: Same version is eligible

- **WHEN** the device runs version `0.7.0` on slot A and the directory
  holds only `obake-0.7.0-b.raucb`
- **THEN** the command selects that file

### Requirement: Device update validates a file bundle unless forced

When `PATH` is a file named `obake-<semver>-<slot>.raucb`, `obake device
update` SHALL refuse the file when `<slot>` is not the inactive boot
slot or when `<semver>` is less than the device's running image version,
unless `--force` is set. With `--force`, the command SHALL push and
apply that file without those checks. A file whose name does not match
the expected pattern SHALL fail unless `--force` is set.

#### Scenario: Wrong-slot file is rejected

- **WHEN** the device's inactive slot is `a` and the operator passes
  `obake-0.8.0-b.raucb` without `--force`
- **THEN** the command fails without copying the file

#### Scenario: Older file is rejected

- **WHEN** the device runs version `0.8.0` and the operator passes
  `obake-0.7.0-a.raucb` for the inactive slot without `--force`
- **THEN** the command fails without copying the file

#### Scenario: Force bypasses slot and semver checks

- **WHEN** the operator passes `--force` with a specific
  `obake-0.7.0-b.raucb` while the inactive slot is `a` and the device
  runs `0.8.0`
- **THEN** the command copies that file and runs
  `obake-update apply --cleanup --reboot` on the staged path

### Requirement: The client config names hosts and a default

The `obake` client config SHALL list hosts, each with an endpoint, a
bearer token, and a certificate fingerprint, and SHALL name the default
host. The client SHALL use that default unless a command names another
host. A command-line endpoint or token SHALL replace the chosen host's
value. A missing file SHALL leave the local Unix socket in use when
that socket exists. When the config file and the local socket are both
absent, the command SHALL fail without sending a request and SHALL tell
the operator to run `obake auth login <ssh-user@host>`. An
unknown host name SHALL fail and SHALL NOT send a request. An HTTPS
request SHALL fail when the peer certificate fingerprint does not match
the stored fingerprint, or when the fingerprint is missing, and that
failure SHALL NOT perform a GraphQL operation or file transfer.

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

- **WHEN** no config file exists, the operator passes no endpoint, and
  the local Unix socket exists
- **THEN** the client uses the local Unix socket

#### Scenario: A missing file with no local socket explains login

- **WHEN** no config file exists, the operator passes no endpoint, and
  the local Unix socket does not exist
- **THEN** the command fails without sending a request and tells the
  operator to run `obake auth login <ssh-user@host>`

#### Scenario: An unknown host is rejected

- **WHEN** the operator names a host that is not in the config
- **THEN** the command fails and no request is sent

#### Scenario: A mismatched certificate is rejected

- **WHEN** the chosen host's stored fingerprint differs from the
  certificate presented by its endpoint
- **THEN** the command fails and no GraphQL operation or file transfer
  is performed

#### Scenario: A missing fingerprint is rejected

- **WHEN** the chosen host's endpoint is HTTPS and the config has no
  fingerprint for that host
- **THEN** the command fails and no GraphQL operation or file transfer
  is performed

### Requirement: GraphQL and file HTTP are served locally and on the network

The service SHALL accept GraphQL and session-file HTTP on a Unix socket
in the `obake` user's runtime directory and on a configured TLS address.
A request to the TLS address SHALL present either the host's bearer
token or a valid browser session cookie. A TLS request with neither
SHALL be rejected and SHALL NOT change the session or work-area files.
A request on the Unix socket SHALL NOT require the token. The TLS
address SHALL NOT accept plaintext HTTP.

#### Scenario: A tokened network client is served

- **WHEN** a client queries the configured TLS address with the bearer
  token
- **THEN** the query result is returned

#### Scenario: A network request without the token is rejected

- **WHEN** a client queries the configured TLS address without the
  bearer token and without a browser session cookie
- **THEN** the request is rejected and the active session is unchanged

#### Scenario: The local socket does not require the token

- **WHEN** a client queries the user Unix socket without a bearer token
- **THEN** the query result is returned

#### Scenario: Network file GET requires the token

- **WHEN** a client GETs a session file on the TLS address without the
  bearer token and without a browser session cookie
- **THEN** the request is rejected and no file bytes are returned

#### Scenario: A browser cookie is served

- **WHEN** a client queries the configured TLS address with a valid
  browser session cookie and without the bearer token
- **THEN** the query result is returned

#### Scenario: Plaintext HTTP on the TLS address is rejected

- **WHEN** a client sends plaintext HTTP to the configured TLS address
- **THEN** the request is rejected and the active session is unchanged

### Requirement: The host serves a stable self-signed certificate

The service SHALL present a self-signed certificate on its TLS
listener. It SHALL create the certificate and private key when they
are absent, and SHALL present the same certificate after a process
restart or a host boot. The certificate SHALL name the system hostname
and that hostname with a `.local` suffix.

#### Scenario: First start creates a certificate

- **WHEN** the service starts and no certificate is stored yet
- **THEN** it creates one and the TLS listener presents it

#### Scenario: Restart keeps the same certificate

- **WHEN** the service starts again after a process restart
- **THEN** the TLS listener presents the same certificate as before

#### Scenario: The certificate names the host

- **WHEN** the system hostname is `studio`
- **THEN** the certificate includes `studio` and `studio.local`

### Requirement: The host credential is local to the socket

`obake auth credential` SHALL print the current bearer, the TLS listen
port, and the certificate fingerprint when it can reach the Unix
socket. The TLS listener SHALL NOT return the bearer or the private
key, including to a client that sends the bearer or a browser cookie.

#### Scenario: The local socket prints the credential

- **WHEN** the operator runs `obake auth credential` on the host
- **THEN** the output includes the bearer, the listen port, and the
  certificate fingerprint

#### Scenario: The network cannot read the credential

- **WHEN** a client on the TLS listener requests the credential with
  the bearer or with a browser cookie
- **THEN** the bearer and the private key are not returned

### Requirement: SSH login stores the host credential

`obake auth login <ssh-user@host>` SHALL use the system SSH client to
read that host's bearer, listen port, and certificate fingerprint.
Success SHALL record an `https` endpoint, the bearer, and the
fingerprint under the host part of the argument, and SHALL set the
default host when none is set. A failed SSH SHALL leave the config
unchanged. Logging in again for one host SHALL leave every other host
unchanged.

#### Scenario: Login writes a usable host entry

- **WHEN** the operator runs `obake auth login obake@studio.local` and
  SSH succeeds
- **THEN** the config contains host `studio.local` with an `https`
  endpoint, the bearer, and the fingerprint
- **AND** a following command to that host is accepted by the service

#### Scenario: Login over a remote command still enrolls

- **WHEN** login reaches the host with a non-interactive SSH command
- **THEN** the config still receives the bearer and the fingerprint

#### Scenario: A failed SSH does not change config

- **WHEN** SSH to the named user and host fails
- **THEN** the client config file is unchanged

#### Scenario: A second login updates only that host

- **WHEN** the config has hosts `studio.local` and `spare.local` and
  login succeeds again for `obake@studio.local`
- **THEN** `studio.local` has the current bearer and fingerprint
- **AND** `spare.local` is unchanged

### Requirement: A device code admits one browser

The TLS login page SHALL show a device code to a browser that has no
session. `obake auth approve <code>` SHALL accept a current code from
the Unix socket or from a client sending the host bearer. The browser
that started that login SHALL then receive a session cookie. A client
that did not start the login SHALL NOT receive a cookie by presenting
the code alone. An unknown or expired code SHALL be rejected and SHALL
NOT set a cookie.

#### Scenario: Approve on the host signs the waiting browser in

- **WHEN** a browser has been shown a device code and the operator runs
  `obake auth approve` with that code on the host
- **THEN** that browser receives a session cookie that authorizes a
  GraphQL query

#### Scenario: Approve from an enrolled CLI signs the browser in

- **WHEN** a laptop CLI holds the host bearer and the operator runs
  `obake auth approve` with the browser's code
- **THEN** that browser receives a session cookie

#### Scenario: The typed code alone does not yield a cookie

- **WHEN** a second client presents a device code it did not start
- **THEN** it receives no session cookie

#### Scenario: An expired code is rejected

- **WHEN** the operator approves a code more than 15 minutes after the
  browser was shown it
- **THEN** approval fails and the browser receives no session cookie

#### Scenario: A restart drops a pending code

- **WHEN** the service restarts after a code is shown and before it is
  approved
- **THEN** approving that code fails

### Requirement: The browser cookie lasts until the bearer changes

A browser session cookie SHALL authorize GraphQL, its subscription
socket, and session-file HTTP on the TLS listener for as long as the
host bearer is unchanged, including after a service restart. The
cookie SHALL NOT authorize approve, rotate, or reading the host
credential.

#### Scenario: The cookie survives a service restart

- **WHEN** a browser holds a session cookie and the service restarts
  without a bearer change
- **THEN** that cookie still authorizes a GraphQL query

#### Scenario: The cookie cannot approve another browser

- **WHEN** a client presents only a valid browser cookie and attempts
  to approve a device code
- **THEN** approval fails and the bearer is unchanged

### Requirement: Rotating the bearer revokes other clients

`obake auth rotate` SHALL replace the host bearer in the running
service when called on the Unix socket or with the current bearer. It
SHALL leave the TLS certificate unchanged. The CLI SHALL write the new
bearer into its selected host entry when that entry exists. Cookies
issued before the rotation, and any other copy of the previous bearer,
SHALL be rejected. Rotate presented with only a browser cookie SHALL
fail and SHALL leave the bearer unchanged.

#### Scenario: Rotate on the host invalidates the old bearer

- **WHEN** the operator runs `obake auth rotate` on the host
- **THEN** the previous bearer is rejected, the new bearer is accepted,
  and the certificate fingerprint is unchanged

#### Scenario: Rotate from a laptop updates that laptop

- **WHEN** an enrolled CLI runs `obake auth rotate` using its stored
  bearer
- **THEN** that host's config entry holds the new bearer and the same
  fingerprint
- **AND** the previous bearer is rejected

#### Scenario: Rotate signs browsers out

- **WHEN** a browser holds a session cookie and the bearer is rotated
- **THEN** that cookie is rejected

#### Scenario: A cookie cannot rotate the bearer

- **WHEN** a client presents only a valid browser cookie and attempts
  to rotate
- **THEN** rotation fails and the current bearer is still accepted

### Requirement: Session file HTTP transfers stream without a body-size cap

HTTP GET and PUT of session work-area bytes SHALL transfer the body as a
sequence of parts so the service need not hold the whole file in memory.
The service SHALL NOT reject a file transfer solely because the body
exceeds a small fixed buffer limit. PUT SHALL still create or replace a
regular file atomically and SHALL NOT leave a partial replace on failure.

#### Scenario: A multi-megabyte file round-trips without a body-size reject

- **WHEN** a client PUTs a regular file larger than 8 MiB into a session
  work area and then GETs that path
- **THEN** the PUT is accepted, the GET body matches the uploaded bytes,
  and neither request fails with a payload-too-large body-limit error

#### Scenario: A failed PUT leaves the prior file intact

- **WHEN** a PUT of a large file fails after some bytes have been
  received
- **THEN** any previously existing file at that path is unchanged

### Requirement: The CLI streams file get and put

`obake file get` and `obake file put` SHALL stream file octets in parts
between disk (or stdout) and HTTP so the CLI need not hold the whole
file in memory. Destinations `.` and `-`, and `--session`, SHALL keep
their existing meaning.

#### Scenario: File put accepts a multi-megabyte local file

- **WHEN** the operator runs `obake file put` with a local file larger
  than 8 MiB
- **THEN** the command completes successfully and the remote work-area
  path holds the same bytes

#### Scenario: File get streams to stdout

- **WHEN** the operator runs `obake file get large.bin -`
- **THEN** the raw bytes are written to stdout as they arrive
