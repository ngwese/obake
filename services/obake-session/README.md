# obake-session

Rig and session daemon for the obake host. Serves GraphQL for launching
shapes, applying links, and restoring sessions across reboot.

## Listeners

| Listener | Path / address | Auth |
|---|---|---|
| Unix socket | `$XDG_RUNTIME_DIR/obake-session/api.sock` | none |
| TCP | `host.yaml` `listen` (default `0.0.0.0:9477`) | `Authorization: Bearer` from token file |

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

Use the separate `obake` binary / crate. It does not link this service.
