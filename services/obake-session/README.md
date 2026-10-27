# obake-session

Rig and session daemon for the obake host. Serves GraphQL for launching
shapes, applying links, and restoring sessions across reboot. Session
work-area bytes are transferred over HTTPS on the same listeners.

## Listeners

| Listener | Path / address | Auth |
|---|---|---|
| Unix socket | `$XDG_RUNTIME_DIR/obake-session/api.sock` | none |
| TLS | `host.yaml` `listen` (default `0.0.0.0:9477`) | bearer or browser cookie |

Plaintext HTTP on the listen address is rejected. The HTTP API is rooted
at `/api/v0/`. GET `/api/v0/` serves GraphiQL v2 (subscriptions on
`/api/v0/ws`).

A browser opens `/api/v0/login`, which shows a device code. `obake auth
approve <code>` on the host (Unix socket) or from an enrolled CLI accepts
it. The browser then keeps a long-lived cookie. That cookie authorizes
GraphQL, the websocket, and file transfer. It does not approve further
browsers, rotate the bearer, or read the credential.

`obake auth rotate` replaces the bearer in the running service. Existing
cookies and other copies of the bearer stop working. The TLS certificate
does not change.

`obake auth credential` on the Unix socket prints JSON
`{"token","port","fingerprint"}`. The TLS listener does not serve that
route. When `XDG_RUNTIME_DIR` is unset the socket is
`/run/user/<uid>/obake-session/api.sock`.

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

A rig is either a single file or a directory package under config:

```
~/.config/obake-session/host.yaml
~/.config/obake-session/rigs/<name>.yaml          # single-file form
~/.config/obake-session/rigs/<name>/rig.yaml      # directory form
~/.config/obake-session/rigs/<name>/work/…        # optional fresh template
~/.config/obake-session/token          # created 0600 if missing
~/.config/obake-session/tls.crt        # self-signed cert, mode 0600
~/.config/obake-session/tls.key        # mode 0600
~/.local/state/obake-session/current
~/.local/state/obake-session/sessions/<id>.yaml
~/.local/share/obake-session/sessions/<id>/work   # mounted at /work
~/shapes/<shape>.sif                   # or host.yaml image_dir
```

Fresh starts copy the package `work/` template into the new session
work directory (or start empty for a single-file rig). Instances see
only `/work` (that session's copy). Fork copies session work; the next
fresh start reseeds from the template. Directory packages can also be
installed with `PUT /api/v0/rigs/{name}/package` (uncompressed tar;
Unix socket or bearer only).

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
