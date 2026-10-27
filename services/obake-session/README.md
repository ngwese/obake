# obake-session

Rig and session daemon for the obake host. Serves GraphQL for launching
shapes, applying links, and restoring sessions across reboot. Session
work-area bytes are transferred over HTTP on the same listeners.

## Listeners

| Listener | Path / address | Auth |
|---|---|---|
| Unix socket | `$XDG_RUNTIME_DIR/obake-session/api.sock` | none |
| TCP | `host.yaml` `listen` (default `0.0.0.0:9477`) | `Authorization: Bearer` from `token` file |

The HTTP API is rooted at `/api/v0/`. GET `/api/v0/` serves GraphiQL v2
(subscriptions on `/api/v0/ws`). On TCP, GraphiQL needs the bearer token
in the request headers.

## Session file HTTP

```
GET  /api/v0/sessions/{id}/files/{*path}
PUT  /api/v0/sessions/{id}/files/{*path}
GET  /api/v0/sessions/current/files/{*path}
PUT  /api/v0/sessions/current/files/{*path}
```

Body is raw octets (`Content-Type: application/octet-stream`), streamed
in parts with no small fixed size cap. Paths are relative to that
session's work area; `..` and absolute paths fail. `current` is the
active session and fails when idle. Listing stays on GraphQL
`sessionFiles`.

## Layout

```
~/.config/obake-session/host.yaml
~/.config/obake-session/rigs/<name>.yaml
~/.config/obake-session/token          # created 0600 if missing
~/.local/state/obake-session/current
~/.local/state/obake-session/sessions/<id>.yaml
~/.local/share/obake-session/rigs/<name>/work
~/.local/share/obake-session/sessions/<id>/work
~/shapes/<shape>.sif                   # or host.yaml image_dir
```

## Run

```bash
obake-session run
# overrides for tests:
obake-session run --config-dir … --state-dir … --share-dir … --runtime-dir … --image-dir …
```

Request logging (GraphQL query/variables, file GET/PUT, method/URI/status):

```bash
RUST_LOG=obake_session::http=debug obake-session run
# Full GraphQL JSON and file body chunks:
RUST_LOG=obake_session::http=trace obake-session run
# or via the user unit:
# Environment=RUST_LOG=obake_session::http=debug
```

`debug` logs compact GraphQL queries and file transfer sizes. `trace` also
dumps full GraphQL request/response JSON and each file body chunk
(UTF-8 text, or hex for binary).

## Rig document

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

Instance keys are PipeWire node names. Keys must not contain `:` or `/`.

## Host profile

```yaml
listen: "0.0.0.0:9477"
environment:
  OBAKE_HOST: qdl
sample_library:
  source: /home/obake/samples
  target: /samples
aliases:
  mixpre:
    card: II
image_dir: /home/obake/shapes
```

## Operator client

Use the separate `obake` binary / crate (`rig ls`, `session start`,
`file get` / `file put`, …). It does not link this service.
