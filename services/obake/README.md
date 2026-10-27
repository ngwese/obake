# obake

GraphQL / HTTP client library and `obake` CLI for `obake-session`. Builds
without PipeWire, JACK, Singularity, or the session crate.

## Config

`$XDG_CONFIG_HOME/obake/config.yaml` (or `~/.config/obake/config.yaml`):

```yaml
default: studio.local
hosts:
  studio.local:
    endpoint: https://studio.local:9477
    token: <bearer>
    fingerprint: sha256:<hex of the certificate>
  spare.local:
    endpoint: https://spare.local:9477
    token: <bearer>
    fingerprint: sha256:<hex of the certificate>
```

Resolution: `--endpoint` / `--token` > `--host` > `default` > local Unix socket.
A missing config file uses the Unix socket when that socket exists. When
neither the file nor the socket is present, the command tells you to run
`obake auth login <ssh-user@host>`. An unknown `--host` fails before
any request. `--config` selects another file. An `https://` host without a
fingerprint, or whose certificate does not match the fingerprint, fails
before a GraphQL operation or file transfer.

```bash
obake auth login obake@studio.local
obake auth approve K7QM-2P9X
obake auth rotate
```

`auth login` runs `ssh` and stores that host's bearer and certificate
fingerprint. The host part of `user@host` is the config name. `auth
approve` accepts a browser device code on the selected host (the Unix
socket when no config is present). `auth rotate` replaces the bearer and
updates the selected host entry; the fingerprint stays. `auth` is the
documented noun that is not a schema entity.

## Output

Commands print a human-readable table (or short status line) by default.
Pass `--json` anywhere to print the GraphQL `data` payload instead.

```bash
obake rig ls
obake --json rig ls
obake session ls --rig quartet --json
```

## Subcommands

Noun-verb form. List with `<noun> ls` (not a plural noun alone).

| Command | Transport |
|---|---|
| `obake rig ls` | GraphQL `rigs` |
| `obake rig describe <name>` | GraphQL `rig(name)` |
| `obake rig create <name> --file …` | GraphQL `installRig` |
| `obake rig delete <name>` | GraphQL `removeRig` |
| `obake shape ls` | GraphQL `shapes` |
| `obake session ls [--rig NAME]` | GraphQL `sessions(rig:)` |
| `obake session describe` | GraphQL `session` (active; fails when idle) |
| `obake session describe <id>` | recorded session |
| `obake session start <rig> [--mode FRESH\|FORK] [--fork-from ID]` | GraphQL `startSession` |
| `obake session stop` | GraphQL `stopSession` |
| `obake session delete <id>` | GraphQL `deleteSession` |
| `obake link set [--links JSON]` | GraphQL `setSystemLinks` |
| `obake file ls [--session ID]` | GraphQL `sessionFiles` |
| `obake file get <src…> <dest> [--session ID]` | HTTP GET `/api/v0/sessions/…/files/…` |
| `obake file put <src…> <dest> [--session ID]` | HTTP PUT `/api/v0/sessions/…/files/…` |
| `obake graph ls` | GraphQL `graph` |
| `obake ravenna set` | GraphQL `setRavennaStreams` (JSON on stdin) |
| `obake watch` | subscriptions |

`file put` / `file get` take one or more sources and a final destination
(cp-style). Sources may be globs or directories; a `.` or directory
destination receives basenames (or relative paths for tree uploads) and
is created when missing. Remote `file get` sources may be globs,
directory prefixes, or `.` (all work-area files). Local get destinations:
`.` preserves remote relative paths under the cwd; `-` streams a single
file to stdout. Omitting `--session` uses the active session. Transfers
stream octets in parts so large work-area files need not fit in RAM.

## Debug logging

```bash
RUST_LOG=obake::client=debug obake session describe
# Full GraphQL JSON and file body chunks:
RUST_LOG=obake::client=trace obake session describe
```

`debug` logs GraphQL query/variables summaries and file HTTP method, URL,
and sizes. `trace` also dumps full GraphQL request/response payloads and
each file transfer chunk (UTF-8 text, or hex for binary).
