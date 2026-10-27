# Proposal

## Why

`obake file put` fails on large files with HTTP 413 because the session
service buffers the whole request body under Axum's default body limit
(~2 MiB), and both the client and server hold entire files in memory.
Operators need to move arbitrary work-area artifacts (samples, packages,
exports) without size caps or RAM proportional to file size.

## What Changes

- Session-file HTTP GET and PUT stream octets in fixed-size parts instead
  of buffering the whole payload.
- Remove the request-body size limit for session-file routes so large
  uploads are accepted.
- The `obake` client streams `file put` from disk and `file get` to a
  destination (file or stdout) without loading the full file into a
  `Vec<u8>`.
- CLI surface (`file get` / `file put`, dest `.` / `-`, `--session`) stays
  the same; only transport behavior changes.
- Headless tests cover a multi-MiB round-trip and confirm neither side
  requires a single in-memory copy of the whole file.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `obake-session`: Add requirements that session-file HTTP and the
  `obake` file CLI stream arbitrary-size bodies in parts, without a
  small fixed body limit and without holding the whole file in memory.
  (Main `openspec/specs/obake-session` is not archived yet; the delta
  uses ADDED requirements so this change can archive independently of
  `add-obake-session`.)

## Impact

- `services/obake-session` HTTP handlers and service write path
- `services/obake` HTTP client and `file get` / `file put` CLI
- READMEs for those crates if they document file transfer limits
- No change to GraphQL, session lifecycle, shapes, OTA, or host image
  layout
