# Tasks

## 1. TLS listener and certificate

- [x] 1.1 Add rustls, a pure-Rust certificate generator, and an HMAC
  crate to the workspace dependencies used by `services/obake-session`
  and `services/obake`, and refresh `services/Cargo.lock`. No shape or
  app pin bump: no shape dependency changes.
  `image/os/buildroot/package/obake-session/obake-session.mk` already
  builds both crates `--locked` and installs both binaries; leave that
  makefile unchanged unless the locked build fails. Verify
  `cargo metadata --locked --manifest-path services/Cargo.toml`
  succeeds and lists the new crates.
- [x] 1.2 Persist a self-signed key and certificate mode 0600 beside the
  token file under the obake-session config dir. Subject alternative
  names are the system hostname and `<hostname>.local`. Reuse the files
  when they already exist. Verify a unit test creates them on first
  call, returns the same SHA-256 fingerprint on the second call, and
  records both DNS names.
- [x] 1.3 Serve the existing axum router with rustls on `host.yaml`
  `listen`. Reject plaintext HTTP on that address. Keep the Unix socket
  unauthenticated. Hold the bearer so a later request can observe a
  replacement. Verify `services/obake-session/tests/milestone1.rs`
  (updated for TLS) accepts a bearer query, rejects a query with no
  credential, rejects plaintext, and still serves the Unix socket
  without a token.

## 2. Browser session and rotation

- [x] 2.1 Accept either the host bearer or the browser cookie on the TLS
  listener. The cookie is HMAC-SHA256 of `obake-browser-v1` under the
  current bearer. Compare both in constant time. A cookie authorizes
  GraphQL, the subscription websocket, and session-file HTTP, and does
  not authorize approve, rotate, or credential readout. Verify a
  headless test: a cookie query succeeds, a request with neither
  credential is rejected, and a cookie cannot rotate.
- [x] 2.2 Add unauthenticated `GET /api/v0/login`,
  `POST /api/v0/login/start`, and `POST /api/v0/login/poll`, plus
  GraphQL `approveLogin`. Pending logins stay in memory for 10 minutes,
  with a cap and a limit on failed approvals. The poll secret is
  required to set the cookie. Verify a headless test: approve from the
  Unix socket and approve with the bearer each set a cookie for the
  client that started the login; a second client that only knows the
  typed code gets no cookie; a code older than 15 minutes and a code
  from before a restart are rejected.
- [x] 2.3 Add `auth rotate` through the running service: new bearer
  written mode 0600, in-memory check updated, certificate untouched,
  mutation returns the new bearer. Mount `obake auth credential` only
  on the Unix socket and teach the CLI to find
  `/run/user/<uid>/obake-session/api.sock` when `XDG_RUNTIME_DIR` is
  unset. Verify a headless test: after rotate the old bearer and old
  cookie fail, the new bearer succeeds, the fingerprint is unchanged,
  and a TLS client cannot read the bearer.
- [x] 2.4 Update `services/obake-session/README.md` so the listener table
  says HTTPS, documents the login and cookie, and documents rotate.
  Verify the README matches the routes and the files next to the token.

## 3. CLI enrollment

- [x] 3.1 Extend `services/obake` host config with an optional
  certificate fingerprint. On HTTPS, fail before a GraphQL operation or
  file transfer when the peer certificate does not match the pin or the
  pin is missing. Verify `services/obake/tests/cli_stub.rs` covers a
  matching pin, a mismatch, and a missing fingerprint, and that a
  missing config file still selects the Unix socket.
- [x] 3.2 Implement `obake auth login <ssh-user@host>` by execing the
  system `ssh` to run `obake auth credential`, then write endpoint,
  bearer, and fingerprint for that host part. Set `default` when empty.
  Leave other hosts and the file unchanged when SSH fails. Write the
  config mode 0600. Verify a headless test using a fake `ssh` on `PATH`:
  success writes a usable `studio.local` entry, a second login updates
  only that host, and a non-zero SSH status leaves the file as it was.
- [x] 3.3 Implement `obake auth approve <code>` and `obake auth rotate`
  against the selected endpoint (Unix socket or stored bearer). After a
  successful rotate, write the new bearer into the selected host entry
  and leave the fingerprint. Verify a stub or live-service test for
  approve, and a test that rotate updates the local config token only.
- [x] 3.4 Update `services/obake/README.md` with the fingerprint field
  and the `auth login`, `auth approve`, and `auth rotate` commands.
  Verify the examples match the noun-verb CLI and the config resolution
  order.

## 4. Headless integration

- [x] 4.1 Add a test that runs the `obake` client against a live
  `obake-session` TLS listener: login's credential JSON, a device-code
  approval, a pinned request, and rotate. Verify
  `cargo test --locked --manifest-path services/Cargo.toml -p obake-session -p obake`
  passes.
