# Design

## Context

See proposal.md for motivation. Today:

- GraphQL `Session` exposes id, rig, mode, phase, links — not instances.
  Instance names live on the rig document and in `ActiveSession.running`
  as Singularity names `obake-<sessionId>-<instance>`.
- `SingularityLauncher` uses `singularity instance run`; CE 4.x writes
  per-instance logs under
  `~/.singularity/instances/logs/<host>/<user>/<name>.{out,err}`.
  Paths are discoverable with `singularity instance list --logs` / `-j`.
- Singularity does not delete those files on stop; they accumulate.
  Chuck verbose output is typically on `.err`; `.out` is often empty.
- Session work-area bytes already use authenticated chunked HTTP on the
  same listeners as GraphQL (`/api/v0/sessions/.../files/...`).

## Goals / Non-Goals

**Goals:**

- Surface rig instance names on the active session GraphQL type and in
  `obake session describe`.
- Stream Singularity `.out` + `.err` for the active session over HTTP,
  with multiplex prefixes and optional follow.
- CLI `obake session log [-f|--follow] [INSTANCE...]`.

**Non-Goals:**

- Serving stopped/historical sessions (files may remain on disk).
- Owning a second log directory under the share tree.
- Rotating or deleting Singularity logs.
- GraphQL subscriptions for log lines.
- Process liveness / exit-code monitoring (separate concern).

## Decisions

### 1. Read Singularity log files; do not capture at launch

**Choice:** Resolve `.out`/`.err` for running instances via
`singularity instance list --logs` (or JSON) keyed by the known
Singularity instance name.

**Why:** Logs already exist; no launcher change; works for shapes
already running. Hostname in the path stays Singularity's problem.

**Alternatives:** Redirect stdio into `share/sessions/<id>/logs/` at
start (cleaner ownership, loses history for current sessions, more
launch plumbing); scrape journalctl (mixed instances, noisy).

### 2. One multiplexed HTTP GET for the active session

**Choice:** Something like
`GET /api/v0/session/logs?follow=0|1&instance=<name>*`
(omit `instance` = all). Response is `text/plain` chunked; each line
prefixed `[<instance>] ` after merging that instance's out+err reads.
Auth matches session-file GET.

**Why:** Matches the file-transfer pattern; one CLI connection; server
owns merge/prefix. Active-only keeps the URL free of session id for v1
(always the current session).

**Alternatives:** Per-instance URLs with CLI N-way merge (harder
ordering); GraphQL subscription (heavier, doesn't match binary/text
file precedent).

### 3. Follow = dump then poll/tail the files

**Choice:** Emit existing file contents (from offset 0), then poll for
growth (or equivalent), writing new bytes as lines with the same
prefix. End the response when the client disconnects or the active
session clears.

**Why:** Singularity appends to ordinary files; no need for a notify
API. Line-oriented prefixing keeps multiplex readable.

**Alternatives:** `inotify` (faster, more platform detail); shell out
to `tail -F` (harder to multiplex cleanly inside the service).

### 4. GraphQL instances on active `session` only

**Choice:** Add `instances { name shape ... }` to the active `Session`
type by loading the session's rig. Recorded `sessions` list stays as
today. CLI `session describe` for the active session prints them;
inactive id describe may resolve the rig by name for the same table
without a new GraphQL field on `SessionRecord`.

**Why:** Spec requires describe to list names; active path is the
common case. Avoids bloating the historical list type.

### 5. Retention = Singularity behavior

**Choice:** Document and leave alone. v1 does not clean up on stop or
session delete.

**Why:** Locked in exploration; avoids inventing policy before it is
needed. Future work can add cleanup or historical serve.

## Risks / Trade-offs

- [Hostname / path drift] → Always resolve via `instance list --logs`
  for running names rather than hardcoding the directory layout.
- [Instance exits but Singularity wrapper stays up] → Logs still
  readable until stop; follow may go quiet. Out of scope for liveness.
- [Large accumulated `.err` on first dump] → Accept for v1; optional
  byte/line window can wait.
- [Journal still duplicates startup lines] → Harmless; CLI uses files.
- [Fake launcher in tests has no real log files] → Tests write fixture
  files or stub a log-path resolver trait.

## Migration Plan

- Ship service + CLI together; old CLIs ignore new GraphQL fields.
- No host image bump required unless packaging a new `obake` /
  `obake-session` binary into the image in the usual way.
- Rollback: revert binaries; Singularity logs unchanged on disk.

## Open Questions

None. Path chosen for apply:
`GET /api/v0/sessions/current/logs?follow=0|1&instance=<name>`
(omit `instance` for all), matching session-file "current" resolution.
