# obake

GraphQL client library and `obake` CLI for `obake-session`. Builds without
PipeWire, JACK, Singularity, or the session crate.

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
obake rigs
obake --json rigs
obake sessions --rig quartet --json
```

## Subcommands

| Command | GraphQL |
|---|---|
| `obake rigs` | `rigs` |
| `obake rig <name>` | `rig(name)` |
| `obake shapes` | `shapes` |
| `obake sessions [--rig NAME]` | `sessions(rig:)` |
| `obake session` | `session` |
| `obake start <rig> [--mode FRESH\|FORK] [--fork-from ID]` | `startSession` |
| `obake stop` | `stopSession` |
| `obake links [--links JSON]` | `setSystemLinks` |
| `obake files <id>` | `sessionFiles` |
| `obake file <id> <path>` | `sessionFile` |
| `obake delete-session <id>` | `deleteSession` |
| `obake rig install <name> --file …` | `installRig` |
| `obake rig remove <name>` | `removeRig` |
| `obake graph` | `graph` |
| `obake ravenna set` | `setRavennaStreams` (JSON on stdin) |
| `obake watch` | subscriptions |
