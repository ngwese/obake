# Design

## Context

Today PUT extracts `axum::body::Bytes` (subject to Axum's default
~2 MiB `DefaultBodyLimit`) and the service writes via
`write_session_file(..., &[u8])`. GET reads the whole file into a
`Vec<u8>` and returns it. The `obake` client builds a `Full` body from a
`Vec`/`&[u8]` and collects the response the same way. See proposal.md
for why that fails on large files.

URL shape, auth, path sanitization, and CLI flags stay as in
`add-obake-session`.

## Goals / Non-Goals

**Goals:**

- Transfer files of arbitrary size over the existing GET/PUT routes.
- Bound peak memory to a small fixed chunk size on both sides.
- Keep atomic replace semantics for PUT (incomplete uploads MUST NOT
  replace the prior file).

**Non-Goals:**

- S3-style multi-request upload sessions, resume, or parallel parts.
- MIME `multipart/form-data`.
- Progress UI, checksums, or content-encoding (gzip).
- Changing GraphQL `sessionFiles` listing.

## Decisions

### 1. One streamed HTTP body, not a multi-request protocol

"Multi-part" means the body is written and read in successive buffer-
sized parts (HTTP/1.1 chunked encoding or a streamed body with
`Content-Length`), not a separate initiate/upload-part/complete API.

**Why:** Same URLs and CLI; fixes 413 and RAM; one connection per
transfer. An S3-style protocol was considered and rejected as extra
state and complexity with no operator benefit for local/LAN transfers.

### 2. Disable the body limit on file routes only

Apply `DefaultBodyLimit::disable()` on the
`/sessions/{id}/files/{*path}` route (or a nested router). Leave the
default limit on GraphQL POST so oversized GraphQL documents still
fail.

### 3. Server PUT: stream to a temp sibling, then rename

Stream the request body to a temp file under the session work area
(same directory as the destination), `fsync`, then `rename` over the
target (reuse `store::atomic_write` patterns / dir fsync). On error or
cancel, delete the temp. Reject directory overwrite and path escape
before creating the temp.

### 4. Server GET: stream from an open file

Open the file after path checks; return
`Content-Type: application/octet-stream` with a streaming body
(`ReaderStream` or equivalent). Prefer `Content-Length` when known.

### 5. Client streams paths and stdout

- `put_file(session, remote, local_path)` opens the file and streams
  it as the request body (chunk size ~1 MiB).
- `get_file(session, remote, writer)` writes response body chunks to a
  `Write`/`AsyncWrite` (file or stdout).
- Drop `get_file -> Vec<u8>` / `put_file(&[u8])` as the primary API;
  tests that need bytes use a temp file or a collecting writer.

### 6. Chunk size

Use a fixed ~1 MiB read/write buffer. Document as an implementation
constant, not a protocol parameter.

## Risks / Trade-offs

- [Interrupted PUT leaves a temp] → Always unlink temp on error; use a
  distinct `.*.tmp` name under the work area.
- [Disk full mid-stream] → Fail the request; leave the previous file
  intact.
- [HTTP/1.1 single connection per transfer] → Acceptable for the CLI;
  no connection pool required in this change.
- [GraphQL still size-limited] → Intentional; file bytes stay off
  GraphQL.

## Migration Plan

Redeploy `obake-session` and `obake` binaries (bata: `~/.local/bin` +
user unit override). No on-disk schema or config migration. Old clients
that buffer small files still work against the new server; new clients
against an old server still hit 413 on large PUTs until the service is
updated.

## Open Questions

(none)
