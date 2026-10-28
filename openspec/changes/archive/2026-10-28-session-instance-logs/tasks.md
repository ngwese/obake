# Tasks

## 1. Active session instances in GraphQL and describe

- [x] 1.1 Extend the GraphQL `Session` type with `instances` (at least
  `name` and `shape`) populated from the active session's rig, and
  verify a milestone or schema test that an active session query
  returns those fields
- [x] 1.2 Update `obake session describe` formatting to list instance
  names (and shapes) for the active session, and verify the CLI table
  or text includes them when a stub/session fixture has instances

## 2. Singularity log path resolution

- [x] 2.1 Add a resolver that maps an active session's rig instance
  names to Singularity `.out`/`.err` paths (prefer
  `singularity instance list --logs` / JSON keyed by
  `obake-<sessionId>-<instance>`), with a testable seam for fakes, and
  verify unit tests cover known-name lookup and missing-instance errors
- [x] 2.2 Reject unknown instance names and the no-active-session case
  before opening files, and verify those error paths in unit tests

## 3. HTTP log stream endpoint

- [x] 3.1 Add `GET /api/v0/sessions/current/logs` (query:
  repeated `instance`, `follow=0|1`) on the same authenticated router
  as session files, streaming `text/plain` with `[name] ` prefixes and
  merged out+err per instance, and verify an integration test dumps
  fixture log bytes for one and for multiple instances
- [x] 3.2 Implement follow: emit existing content then keep reading as
  files grow until client disconnect or the active session clears, and
  verify a test that appends bytes after connect sees them on the
  response (or a unit test of the follow reader)
- [x] 3.3 Confirm TLS requests without bearer/cookie are rejected and
  the Unix socket path works without a token, matching file GET auth,
  via existing auth/router test patterns

## 4. CLI `session log`

- [x] 4.1 Add `obake session log [--follow|-f] [INSTANCE...]` that GETs
  the log endpoint and writes the body to stdout, and verify a CLI stub
  or integration test issues the expected URL/query for zero, one, and
  many instance args with and without `--follow`
- [x] 4.2 Document the command in `services/obake/README.md` (session
  verb table) and verify the README mentions `session log` and describe
  instance listing

## 5. Integration check

- [x] 5.1 On a host with a running tone (or space) session, run
  `obake session describe` and `obake session log` / `obake session log
  -f <instance>` and verify instance names appear and log lines from
  Singularity files stream to stdout
