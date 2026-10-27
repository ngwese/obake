# Spec Delta

## ADDED Requirements

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

## MODIFIED Requirements

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
