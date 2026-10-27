# Tasks

## 1. Package discovery and paths

- [x] 1.1 Update `services/obake-session` path helpers so a rig is
  `rigs/<name>.yaml` or `rigs/<name>/rig.yaml` with optional
  `rigs/<name>/work/`, and stop treating `share/rigs/<name>/work` as a
  live area. Verify unit coverage for both path forms and that a name
  conflict (`<name>.yaml` and `<name>/`) is detected.
- [x] 1.2 Extend `load_rig` / `load_rigs` to load both forms and fail
  closed on a name conflict. Verify listing returns both a single-file
  and a directory package by name, and that a conflict fixture errors.

## 2. Session work seeding and mounts

- [x] 2.1 On fresh start, create `share/sessions/<id>/work`, copy the
  package `work/` template into it when present (else leave empty), and
  bind only that directory at `/work`. Remove the `/session` bind and
  any use of `share/rigs/<name>/work` as a mount. Verify milestone or
  unit tests: template files appear under session work, the package
  template is unchanged after an edit, and a second fresh start reseeds
  from the template.
- [x] 2.2 Keep fork as a copy of the source session work directory and
  restore/restart as the same session work directory. Verify fork
  carries edited files and does not write the package template; verify
  restore mounts the existing session work at `/work` only.

## 3. Install, remove, and package transfer

- [x] 3.1 Make install/remove handle both forms under the config rigs
  tree (replace switches form by removing the other path). Verify
  GraphQL or service tests install a YAML document, install a directory
  package with a `work/` file, remove by name, and refuse remove while
  that rig's session is active.
- [x] 3.2 Add a host-facing transfer for directory packages (HTTP PUT of
  a tar, or equivalent) so a remote CLI can install a local directory
  tree, rejecting path escapes on extract. Verify a round-trip install
  of a small fixture tree and a rejection of an escaping entry.

## 4. CLI: init and create from path

- [x] 4.1 Add `obake rig init <name>` that writes a local `<name>/rig.yaml`
  and `work/` without contacting the service. Verify the scaffold exists
  and a second init on a non-empty path fails or no-ops as documented.
- [x] 4.2 Extend `obake rig create` (or install) to accept a file or
  directory path and install through the service API. Verify a file
  path still installs a single-file rig and a directory path installs a
  package whose fresh start seeds `work/` files.
- [x] 4.3 Update `services/obake/README.md` and
  `services/obake-session/README.md` for package layout, mounts, and
  `rig init` / create-from-path. Verify the documented paths match the
  implementation.

## 5. Demo packages

- [x] 5.1 Convert `demo/obake-session/rigs/{lead,space,tone}` to
  directory packages (`rig.yaml` + `work/` with the former top-level
  seed `.ck` files), delete `demo/obake-session/work/`, and rewrite
  `demo/obake-session/README.md` install steps to copy packages into
  `~/.config/obake-session/rigs/` only. Verify the README layout matches
  the tree and that no demo path still references `share/rigs/*/work`
  seeding. No shape pin bump (no shape or image dependency change).

## 6. Headless verification

- [x] 6.1 Run
  `cargo test --locked --manifest-path services/Cargo.toml -p obake-session -p obake`
  and fix failures until it passes.
