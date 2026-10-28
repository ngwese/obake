# Spec Delta: obake-session

## ADDED Requirements

### Requirement: Active session instance logs stream over HTTP

The service SHALL stream the active session's instance stdout and
stderr from Singularity instance log files over HTTP on the same
listeners as GraphQL and session-file transfer. GraphQL SHALL NOT
carry log body bytes. When no session is active, the request SHALL
fail. The service SHALL NOT invent a separate log store; retention
SHALL be whatever Singularity already keeps on disk. Stopped sessions
SHALL NOT be served by this endpoint in this change.

#### Scenario: Existing log bytes are returned

- **WHEN** the active session has instance `tone` and that instance's
  Singularity log files contain lines
- **THEN** an HTTP GET of the active session log stream for `tone`
  returns those lines as a text body

#### Scenario: No active session fails

- **WHEN** no session is active and a client requests the log stream
- **THEN** the request fails and no log bytes are returned

#### Scenario: A stopped session is not served

- **WHEN** a previously active session has been stopped and its
  Singularity log files still exist on disk
- **THEN** a log stream request that names that stopped session fails
  or is not offered; only the active session is served

### Requirement: Log streams multiplex and filter by instance

A log stream request with no instance names SHALL include every
instance of the active session's rig, each line prefixed with the
rig instance name. A request with one or more instance names SHALL
include only those instances, with the same prefix form. An unknown
instance name SHALL fail the request. Stdout and stderr for an
instance SHALL appear in one stream for that instance.

#### Scenario: All instances are multiplexed

- **WHEN** the active session's rig has instances `lead` and `harmony`
  and a client requests logs with no instance filter
- **THEN** the body includes lines prefixed for `lead` and lines
  prefixed for `harmony`

#### Scenario: Multiple instance names filter the stream

- **WHEN** the active session's rig has `lead`, `harmony`, and `space`
  and a client requests logs for `lead` and `space`
- **THEN** the body includes prefixed lines for `lead` and `space`
  and does not include `harmony`

#### Scenario: An unknown instance name fails

- **WHEN** a client requests logs for instance `missing` and the
  active rig has no such instance
- **THEN** the request fails and no log bytes are returned

### Requirement: Log follow keeps reading new bytes

A follow log request SHALL first emit bytes already present in the
selected Singularity log files, then SHALL keep the HTTP response open
and emit new bytes as those files grow, until the client disconnects
or the service ends the stream because the session is no longer
active.

#### Scenario: Follow emits new lines after startup

- **WHEN** a client opens a follow log stream for instance `tone` and
  that instance later writes another line to its Singularity log
- **THEN** that line appears on the still-open HTTP response

#### Scenario: Follow ends when the session stops

- **WHEN** a follow log stream is open for the active session and that
  session is stopped
- **THEN** the HTTP response ends

### Requirement: Network log GET requires the same credentials as files

A log stream request on the TLS address SHALL present either the host
bearer token or a valid browser session cookie. A TLS request with
neither SHALL be rejected and SHALL NOT return log bytes. A request
on the Unix socket SHALL NOT require the token.

#### Scenario: Network log GET without credentials is rejected

- **WHEN** a client GETs the log stream on the TLS address without the
  bearer token and without a browser session cookie
- **THEN** the request is rejected and no log bytes are returned

#### Scenario: The local socket streams logs without a token

- **WHEN** a client GETs the log stream on the user Unix socket
  without a bearer token while a session is active
- **THEN** the stream body is returned

### Requirement: The CLI streams session logs

`obake session log` SHALL request the active session log HTTP stream
and write it to stdout. With no instance arguments it SHALL request
every instance. With one or more instance arguments it SHALL request
those names. `--follow` SHALL request follow mode.

#### Scenario: Session log dumps every instance

- **WHEN** a session is active and the operator runs
  `obake session log` with no instance names and without `--follow`
- **THEN** the client performs an HTTP GET of the multiplexed log
  stream and writes the body to stdout

#### Scenario: Session log follows selected instances

- **WHEN** a session is active and the operator runs
  `obake session log --follow lead harmony`
- **THEN** the client performs a follow HTTP GET filtered to `lead`
  and `harmony` and writes arriving bytes to stdout until interrupted

#### Scenario: Session describe lists instance names

- **WHEN** a session is active and the operator runs
  `obake session describe` with no id
- **THEN** the output lists each instance name of that session's rig

## MODIFIED Requirements

### Requirement: GraphQL reads rigs and the session

The service SHALL serve GraphQL queries `rigs`, `rig(name)`,
`session`, and `sessions`. `session` SHALL be the active session or empty.
`sessions` SHALL list recorded sessions with id, rig name, mode, and
fork source, and SHALL accept a rig name that limits the list to
that rig. Session mode SHALL be `FRESH` or `FORK`. The active
`session` result SHALL include each instance of that session's rig,
at least by name and shape.

#### Scenario: The active session is readable

- **WHEN** a client queries `session` while a session is active
- **THEN** the result includes that session's id, rig name, mode,
  phase, system links, per-link status, and the rig's instances by
  name and shape

#### Scenario: Idle has no active session

- **WHEN** a client queries `session` after a stop
- **THEN** the result is empty

#### Scenario: A rig lists only its sessions

- **WHEN** two recorded sessions are for rig `quartet` and one is
  for rig `duo`
- **THEN** `sessions` for `quartet` returns those two and not the `duo`
  session

### Requirement: The CLI uses noun-verb commands

`obake` subcommands SHALL follow `<noun> <verb>` by default. Nouns SHALL
be schema entities. Verbs SHALL include `ls`, `start`, `stop`, `create`,
`delete`, `get`, `describe`, `init`, and `log`. A plural noun SHALL NOT be a
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

#### Scenario: Session log is a noun-verb command

- **WHEN** the operator reads or follows active session instance logs
- **THEN** the command is `obake session log`
