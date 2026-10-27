# Proposal

## Why

Rigs are a single YAML file today, while seed files live in a separate
share-tree path that demo scripts copy by hand. That split hides the
package boundary, conflates a shared "piece" mount with session work,
and makes a rig awkward to version, share, or scaffold. Sessions are
supposed to encapsulate a take; a fresh start must not inherit edits
from a prior take.

## What Changes

- **BREAKING**: Drop the shared rig working directory mounted at
  `/work` and the second writable area at `/session`. Instances see
  only `/work`, which is that session's work directory.
- **BREAKING**: A fresh session seeds `/work` from the rig package's
  optional `work/` template (or starts empty for a single-file rig).
  It does not reuse a shared piece across sessions. Restart and fork
  keep or copy that session's work; the next fresh start uses the
  template again.
- Accept a rig as either `rigs/<name>.yaml` or
  `rigs/<name>/rig.yaml` with an optional `work/` template beside it.
  Packages live under the config tree; session bytes stay under share.
- Add `obake rig init <name>` to scaffold a local directory package.
- Update `demo/obake-session` to the directory form and simplify host
  install instructions (no separate share seed step).
- Deferred: a `shapes/` directory inside a package for rig-local
  images.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `obake-session`: Rig package layout (file or directory), session
  work seeding from an optional template, single `/work` mount,
  `obake rig init`, and demo packaging.

## Impact

- `services/obake-session`: path layout, load/install/remove, fresh
  and fork start, launch binds, tests, README.
- `services/obake`: `rig init`, `rig create`/`install` for a file or
  directory path, README.
- `demo/obake-session`: convert `lead`, `space`, and `tone` to
  directory packages; retire the top-level `work/` seed tree.
- Persistent layout on hosts: existing
  `~/.local/share/obake-session/rigs/<name>/work` is no longer used as
  a live mount; operators reinstall packages under config. No OTA or
  shape pin bump. No image makefile change unless a locked build fails.
