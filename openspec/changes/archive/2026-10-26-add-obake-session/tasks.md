# Tasks

## 1. Workspace

- [x] 1.1 Add `services/Cargo.toml` as a workspace of `obake-ravenna` and
  `obake-session`, move the Rust 1.99.1 pin to `services/rust-toolchain.toml`,
  and put shared versions of tokio, clap, env_logger, log, serde, and
  thiserror in `[workspace.dependencies]`. Verify `cargo test -p obake-ravenna`
  from `services/` passes and `services/obake-ravenna/Cargo.lock` is no longer
  the lockfile in use.
- [x] 1.2 Create `services/obake-session/` with clap, env_logger, tokio,
  async-graphql, async-graphql-axum, axum, and serde_yaml, and a `run`
  command that serves the design schema on a Unix socket. Verify
  `cargo test -p obake-session` passes and a headless client can execute
  `{ rigs }` against that socket.
- [x] 1.3 Add workspace member `services/obake/`, a library and an `obake`
  binary, with no dependency on `obake-session`. Verify `cargo build -p obake`
  succeeds and `cargo tree -p obake` does not contain pipewire, jack,
  singularity, or `obake-session`.
- [x] 1.4 Point the `obake-ravenna` Buildroot package at the workspace and
  `-p obake-ravenna`, without changing its installed unit or config seed.
  Verify the package makefile does not select `obake-session` and does not
  change the `chuck` or `rnbo-runner` version pins in `shapes/`.

## 2. Milestone 1 — one ChucK session

- [x] 2.1 Parse `rigs/<name>.yaml` and `host.yaml` as in the design,
  including one `chuck` instance, ordered roles, and rejection of a duplicate
  key or a key containing `:` or `/`. Verify unit tests accept the design's
  `lead` example and reject those two names.
- [x] 2.2 Persist a session under
  `/home/obake/.local/state/obake-session/` with `fsync` before success, and
  create `/work` and `/session` mounts for the rig and session
  directories. Verify a test kills the process after a successful fresh start
  and the reloaded snapshot still names that session, rig, and empty
  system-link list.
- [x] 2.3 Start and stop one `chuck` instance with `singularity instance`,
  the runtime-directory bind, `XDG_RUNTIME_DIR`, and `PIPEWIRE_PROPS` setting
  `node.name` to the instance key. Verify a fake launcher records those
  arguments and `stopSession` records an idle `current` file.
- [x] 2.4 Implement fresh and fork. Verify a test whose rig directory
  already holds `lead.ck` keeps that file, gives the fresh session an empty
  work area, and gives a fork a new id, the same system links, a copied work
  area, and the original rig directory.
- [x] 2.5 Restore the recorded session on startup, and stay idle when the
  recorded state is stopped. Verify both cases against the snapshot fixtures
  without starting a real `shapes/chuck/chuck.def` image.
- [x] 2.6 Implement `sessions(rig:)`, `sessionFiles`, `sessionFile`, and
  `deleteSession` for the session work area. Verify a GraphQL fixture lists
  only `quartet` sessions, returns `take.wav` bytes, rejects `../lead.ck`,
  deletes an inactive session without removing the rig directory, and
  rejects deletion of the active session.

## 3. Milestone 1 — Ravenna wiring

- [x] 3.1 Apply session system links of `{ node, port }` with a fake `pw-link`,
  report `PENDING` until both ports exist, and disconnect a link removed from
  the session. Verify a fixture for `lead` port `outport 1` to
  `obake-out-1:send_1` covers pending, applied, and removal, and that the
  rig YAML is unchanged.
- [x] 3.2 Expose those links through `setSystemLinks`. Verify a GraphQL test
  replaces the session's system links with one Ravenna target and a following
  `session` query returns that target.

## 4. Milestone 1 — query rigs and shapes

- [x] 4.1 Serve `rigs`, `rig(name)`, and `shapes` from the files
  and the compiled profiles. Verify a headless query lists an installed
  rig with roles and links, returns nothing for an unknown rig
  name, and lists `chuck` and `rnbo-runner` as launchable only when
  `/home/obake/shapes/<shape>.sif` (or `image_dir`) exists.
- [x] 4.2 Keep `siren` and `mod-host` out of the launchable set. Verify the
  `shapes` query marks them not launchable or omits them even when a SIF of
  that name is present, and `startSession` for a rig that names either
  shape fails without stopping the active session.

## 5. Milestone 1 — MixPre wiring

- [x] 5.1 Resolve host-profile alias `mixpre` (`card: II`) to the live ALSA
  node and link the requested port. Verify a fixture applies
  `mixpre` / `playback_FL` while that card is present, reports the link
  pending while it is absent, and does not write the alias into the rig
  file.

## 6. Milestone 1 — several shapes

- [x] 6.1 Launch two `chuck` instances and one `rnbo-runner` instance, each
  with its own node name, and map roles through the shape profiles (`outport
  n` / `inport n`, and `outn` / `inn`). Verify a test starts `lead`,
  `harmony`, and `space` and records three distinct node names.
- [x] 6.2 Apply rig role links, including two links onto one input.
  Verify `lead.melody` and `harmony.pad` both target `space.wet`, the stored
  link text contains no shape port string, and both links are applied when
  the ports exist.
- [x] 6.3 Confirm `shapes/chuck/chuck.def` and
  `shapes/rnbo-runner/rnbo-runner.def` are unchanged, including their app
  version pins. Verify `git diff -- shapes/chuck shapes/rnbo-runner` is empty.

## 7. Milestone 1 — CLI

- [x] 7.1 Add client functions and `obake` subcommands for every milestone 1
  query and mutation (`rigs`, `rig`, `shapes`, `sessions`,
  `session`, `start`, `stop`, `links`, `files`, `file`, `delete-session`).
  Verify a headless test serves a stub GraphQL endpoint, runs `obake`
  against it, and records one GraphQL operation per subcommand, without
  starting `obake-session`. (Superseded CLI shape: task 12.1.)
- [x] 7.2 Load hosts from `$XDG_CONFIG_HOME/obake/config.yaml`, or from the
  file named by `--config`. Each host has an `endpoint` and a `token`, and
  `default` names one of them. Verify a temp file with `studio` and `spare`
  sends `studio` when no host is named, sends `spare` for `--host spare`,
  lets `--endpoint` replace the chosen host, uses the Unix socket when the
  file is missing, and fails without a request for an unknown host name.
- [x] 7.3 Point `obake` at a running `obake-session` socket and verify `start`
  makes that service report an active session. Verify this test is not part
  of `cargo test -p obake`.
- [x] 7.4 Serve GraphQL on `host.yaml` `listen` (default `0.0.0.0:9477`) and
  require `Authorization: Bearer` matching
  `/home/obake/.config/obake-session/token`. Create that file, mode `0600`,
  only when it is missing. Verify a TCP query without the token is rejected
  and does not change the session, a TCP query with the token succeeds, and
  the Unix socket still succeeds without a token. Verify `obake` with a
  client config whose `default` host names that endpoint and token performs the same
  `start` as 7.3.

## 8. Milestone 2 — remaining session contract

- [x] 8.1 Serve subscriptions `session` and `links` on the Unix socket and on
  the TCP listener, and add `obake watch`. Verify a test subscriber receives a new
  session after `startSession` and receives `APPLIED` when a pending
  fixture's ports appear.
- [x] 8.2 Mount the host profile's sample library read-only and let an
  instance environment value replace the host value. Verify a fake launch
  records the read-only bind and the instance's `OBAKE_HOST`.
- [x] 8.3 Add `image/os/buildroot/package/obake-session/` and select it from
  `image/os/buildroot/package/obake-runtime/Config.in`. Build
  `-p obake-session` and, separately, `-p obake`. Install an
  `obake` user unit ordered after `pipewire.service`, and a tmpfiles seed
  that creates the home directories only when missing. Verify
  `image/verification/guest/obake-verify.sh` accepts a runtime image where
  `obake-session` is an active user service, `obake` is a separate
  executable, the service is not a system service, and a second boot does
  not rewrite an existing rig file.
- [x] 8.4 Document the socket and the YAML files in
  `services/obake-session/README.md`, and the CLI subcommands in
  `services/obake/README.md`. Verify the `obake` README's `sessions`
  example matches the stub test in 7.1.

## 9. Milestone 3 — rig documents over GraphQL

- [x] 9.1 Implement `installRig` and `removeRig` on the rig
  directory, and the `obake rig install` and `rig remove`
  subcommands. Verify an installed `duo` document appears
  in `rigs` and can be started, and removing the active rig fails
  with that session still running. (Superseded CLI verbs: task 12.1
  `rig create` / `rig delete`.)

## 10. Milestone 4 — live graph

- [x] 10.1 Implement the `graph` query from the ports the reconciler can see,
  and the `obake graph` subcommand. Verify a running `lead`
  fixture appears with its current ports, and the result changes when a port
  disappears without a write to the session file.

## 11. Milestone 5 — Ravenna stream replacement

- [x] 11.1 Implement `setRavennaStreams` by replacing sources and sinks in
  `/home/obake/.config/obake-ravenna/` and reloading that user service, and
  add `obake ravenna set`. Verify a test with a fake config
  writes one source and one sink, does not restart PipeWire, and returns an
  error without creating the directory when the Ravenna service is absent.

## 12. CLI noun-verb UX and HTTP session files

- [x] 12.1 Restructure `obake` to `<noun> <verb>`: `rig ls|describe|create|delete`,
  `shape ls`, `session ls|describe|start|stop|delete`, `link set`,
  `file ls|get|put`, `graph ls`, keeping `ravenna set` and `watch` as
  exceptions. Map former plurals (`rigs`, `shapes`, `sessions`) to
  `<noun> ls`. Verify `obake --help` and a stub GraphQL test exercise each
  new command path, and that `session describe` with no id describes the
  active session.
- [x] 12.2 Serve HTTP GET and PUT for session work-area bytes on the Unix
  socket and TCP listener at `/sessions/{id|current}/files/{*path}`, with
  the same bearer rules as GraphQL. Remove GraphQL `sessionFile` content
  transfer; keep `sessionFiles` for listing. Verify GET returns raw bytes,
  PUT writes a file, `../` fails, TCP without a token is rejected, and
  Unix without a token succeeds.
- [x] 12.3 Implement `obake file get <path> <dest> [--session <id>]` and
  `obake file put <local> <dest> [--session <id>]` over HTTP(S): `.`
  uses the remote basename, `-` streams to stdout, omitted `--session`
  uses the active session. Verify a headless test against a stub HTTP
  endpoint covers get-to-file, get-to-stdout, put, and `--session`.
- [x] 12.4 Update `services/obake/README.md` and
  `services/obake-session/README.md` for the noun-verb CLI and the file
  HTTP routes. Verify the README examples match the commands in 12.1.
