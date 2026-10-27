# Spec Delta: obake-session

## MODIFIED Requirements

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

### Requirement: The CLI uses noun-verb commands

`obake` subcommands SHALL follow `<noun> <verb>` by default. Nouns SHALL
be schema entities. Verbs SHALL include `ls`, `start`, `stop`, `create`,
`delete`, `get`, `describe`, and `init`. A plural noun SHALL NOT be a
top-level command; listing SHALL use `<noun> ls`. Documented exceptions
MAY omit the pattern when a flat verb would be unclear. The noun `auth`
is a documented exception and is not a schema entity.

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

#### Scenario: Rig init scaffolds a local package

- **WHEN** the operator runs `obake rig init tone`
- **THEN** a local directory `tone/` is created with `rig.yaml` and an
  optional `work/` tree, without contacting the session service

## ADDED Requirements

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
