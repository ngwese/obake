# Tasks

## 1. Service streaming HTTP

- [x] 1.1 Add a service API that streams a session work-area file to an
  async writer / returns a readable file handle after path checks, and
  that streams a PUT body into a temp sibling then atomically renames.
  Verify unit coverage for path escape, directory overwrite reject, and
  a successful write that matches input bytes.
- [x] 1.2 Change Axum GET/PUT handlers for
  `/sessions/{id|current}/files/{*path}` to stream request and response
  bodies, and apply `DefaultBodyLimit::disable()` on those routes only.
  Verify a headless HTTP test PUTs and GETs a payload larger than 8 MiB
  successfully and that a GraphQL POST still rejects an oversized body
  under the default limit (or remains unchanged if GraphQL keeps its
  own limit).
- [x] 1.3 Update `services/obake-session/README.md` to note that file
  GET/PUT stream without a small body-size cap. Verify the README
  matches the routes and auth rules still documented there.

## 2. Client streaming CLI

- [x] 2.1 Replace `Client::get_file` / `put_file` buffered `Vec`/`&[u8]`
  APIs with streaming path- and writer-based methods (~1 MiB chunks)
  for both HTTP and Unix endpoints. Verify a stub HTTP test round-trips
  an >8 MiB temp file without collecting the whole body into one
  allocation on the client API surface.
- [x] 2.2 Wire `obake file get` / `file put` to the streaming client
  (dest `.` / `-` / path unchanged). Verify stub or integration coverage
  for get-to-file, get-to-stdout, and put of a multi-MiB file.
- [x] 2.3 Update `services/obake/README.md` file get/put notes for
  streaming large files. Verify examples still match the noun-verb CLI.
