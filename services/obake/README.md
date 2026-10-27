# obake

GraphQL / HTTP client library and `obake` CLI for `obake-session`. Builds
without PipeWire, JACK, Singularity, or the session crate.

## Config

`$XDG_CONFIG_HOME/obake/config.yaml` (or `~/.config/obake/config.yaml`):

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

Resolution: `--endpoint` / `--token` > `--host` > `default` > local Unix socket.
A missing config file is fine (Unix socket). An unknown `--host` fails before
any request. `--config` selects another file.

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
