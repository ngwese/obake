# Design

## Context

Today a rig is `~/.config/obake-session/rigs/<name>.yaml`. Launch binds
`share/rigs/<name>/work` at `/work` and `share/sessions/<id>/work` at
`/session`. Fresh reuses the shared piece and starts an empty take;
fork copies the take and keeps the piece. Demo material seeds the
share tree by hand. Relative shape args are rewritten to `/work/...`.
GraphQL `installRig` writes a YAML document; the CLI is
`obake rig create --file`.

## Goals / Non-Goals

**Goals:**

- Express a rig as a single file or as `<name>/rig.yaml` with an
  optional `work/` template under the config tree.
- Seed each new session's work directory from that template (or empty).
- Mount only that session work at `/work`; drop `/session`.
- Keep restart and fork on the session work directory so encapsulation
  holds across takes.
- Scaffold packages with `obake rig init`.
- Convert `demo/obake-session` to directory packages.

**Non-Goals:**

- Rig-local `shapes/` build or lookup (directory name reserved only).
- Promoting session edits back into the package template.
- Changing role/link schema or shape launch semantics beyond binds.
- OTA or shape version bumps.

## Decisions

### 1. Config holds the package; share holds session work only

```
~/.config/obake-session/rigs/
  duo.yaml                 # single-file form
  tone/
    rig.yaml
    work/tone.ck           # template (not mounted)

~/.local/share/obake-session/sessions/<id>/work/
  tone.ck                  # seeded copy; mounted at /work
```

Stop using `share/rigs/<name>/work` as a live area. Leave orphaned
share trees alone on upgrade; they are unused.

**Why:** Matches host.yaml vs session state. Packages are portable
units under config; takes stay under share.

### 2. Discovery and name conflicts

`load_rigs` accepts `*.yaml` files and subdirectories that contain
`rig.yaml`. The name is the file stem or the directory name. If both
`<name>.yaml` and `<name>/` exist, load and install fail closed.

**Why:** Silent preference would hide operator mistakes when migrating.

### 3. Fresh / fork / restart

| Mode | Session work |
|---|---|
| Fresh | New dir; copy package `work/` if present, else empty |
| Fork | New dir; recursive copy of source session work |
| Restart / restore | Same session id and same work dir |

Template copy is host-side only. Instances never see the package tree.

**Why:** Session encapsulation. Relative args keep resolving under
`/work` without a second mount.

### 4. Install and remove

- Single-file: keep writing `rigs/<name>.yaml` (GraphQL document path).
- Directory: install by placing `rigs/<name>/` with at least `rig.yaml`.
  The CLI `obake rig create --path <file-or-dir>` reads a local path;
  for a directory it transfers the tree to the host (tar over a new
  HTTP PUT on the Unix/TLS API, or an equivalent mutation that writes
  the tree). Replace replaces the prior form of that name after
  removing the other form if present was rejected — replace of the
  same form overwrites; switching form removes the old path then
  writes the new one.
- Remove deletes `rigs/<name>.yaml` or `rigs/<name>/` as installed.
  Active-session guard unchanged.

**Why:** Remote CLI already speaks HTTP for files; a package tar fits
that path without inventing per-file GraphQL.

### 5. `obake rig init`

Local-only scaffold (no service call):

```
obake rig init <name> [--path <dir>]
-> <name>/rig.yaml    # minimal instances: {} / links: []
-> <name>/work/       # empty directory (or .gitkeep)
```

**Why:** Same role as `cargo new` for the shareable unit.

### 6. Demo

Convert `lead`, `space`, and `tone` to directory packages under
`demo/obake-session/rigs/<name>/`, move seed `.ck` files into each
package `work/`, delete the top-level `work/` tree, and rewrite the
README install steps to copy package trees into
`~/.config/obake-session/rigs/`.

## Risks / Trade-offs

- [Existing hosts still have share/rigs/.../work] → Unused after
  deploy; document reinstall of packages under config. No automatic
  migration of piece files into templates.
- [Tar install size] → Acceptable for demo-scale trees; reject paths
  that escape the package root on extract.
- [Requirement titles still say "piece" / "both work areas"] → Spec
  bodies change; titles stay for MODIFIED matching until a later
  rename pass.
- [Switching file <-> directory form] → Explicit replace removes the
  other path so the conflict rule stays true.

## Migration Plan

1. Deploy service and CLI together.
2. Reinstall demo (or operator) packages as directory trees under
   config; stop seeding `share/rigs/*/work`.
3. Existing recorded sessions keep their share session work dirs;
   restore still mounts that dir at `/work` only.
4. Rollback needs the previous dual-mount service; new directory
   packages are unread by the old loader until converted back to
   single YAML plus a manual share seed.
