# Design

## Context

See proposal.md for why. Today `obake-session` accepts plaintext TCP on
`host.yaml` `listen` (default `0.0.0.0:9477`) and compares
`Authorization: Bearer` to a UUID in `~/.config/obake-session/token`
(mode 0600, created on first TCP bind). The Unix socket skips that
check. The `obake` client speaks raw TCP through hyper and stores
`endpoint` plus `token` in `~/.config/obake/config.yaml`. The token
string is fixed in the axum `AppState` for the life of the process.
The host image already installs `/usr/bin/obake` and
`/usr/bin/obake-session`. Non-interactive `ssh host cmd` does not start
a PAM session, so `XDG_RUNTIME_DIR` is often unset, while linger still
provides `/run/user/<uid>/`.

## Goals / Non-Goals

**Goals:**

- Terminate TLS on the existing listen address and pin that certificate
  from an SSH login.
- Enroll a laptop CLI and a phone or tablet without a second identity
  provider.
- Make bearer rotation take effect in the running service and drop
  every browser cookie with it.

**Non-Goals:**

- Per-browser revocation, accounts, or token scopes.
- A public CA, client certificates, or installing a trust anchor on
  phones.
- Regenerating the TLS key as part of `auth rotate`.
- Changing rig, session, or shape documents.

## Decisions

### 1. rustls on the current listen address

Keep the `listen` field. Serve HTTPS there with rustls in front of the
existing axum router. Do not open a second port and do not accept
plaintext HTTP on it. Generate the key and certificate with a pure-Rust
generator on first start, write them mode 0600 beside the token file,
and reuse them after restart. Subject alternative names are the system
hostname and `<hostname>.local`.

**Why:** One listener, no OpenSSL in the session or CLI build, and the
files sit on the user partition so an OTA slot switch keeps the same
certificate. Renaming the host with `hostnamectl` does not rewrite an
existing certificate; the operator removes the cert files and restarts,
then runs login again.

**Alternative:** A public CA. The host is typically a `.local` name on
a LAN, with no inbound 443.

### 2. Pin the certificate, not a CA

The fingerprint is `sha256:` plus the hex SHA-256 of the DER
certificate. `obake auth login` stores it. The HTTPS client checks that
value with a rustls verifier and does not require the certificate to
chain to a system CA. A missing or differing pin fails before any
GraphQL operation or file transfer. Unix sockets stay plaintext.

**Why:** SSH is the trust anchor. Trust-on-first-use would accept a
stranger on the LAN. Pinning the whole certificate means a replaced
cert fails closed even when the key is reused.

### 3. System `ssh` for login

`obake auth login <ssh-user@host>` execs the local `ssh` with the
remote command `obake auth credential`, with `$HOME/.local/bin` on
`PATH` so a user-installed binary is found when the remote shell's
default path is only `/usr/bin` and `/bin`. Stdout is one JSON object:
`token`, `port`, `fingerprint`. The host part of the argument is the
config key and the URL host (`https://studio.local:9477`). The first
success sets `default` when it is empty. A later login replaces that
host's token and fingerprint only. SSH's own exit status, agent,
config, and `known_hosts` are left alone (no
`StrictHostKeyChecking=no`). The config file is mode 0600.

`obake auth credential` reads the running service over the Unix socket.
When `XDG_RUNTIME_DIR` is unset it uses
`/run/user/<uid>/obake-session/api.sock`. The same fallback applies to
`approve` and `rotate` on the host. The credential route exists only on
the Unix router, so the TLS listener cannot return the bearer or the
private key.

**Why:** The operator's SSH setup already works. A non-interactive
remote command still finds the socket because linger created
`/run/user/<uid>`.

### 4. One derived browser cookie

The cookie value is HMAC-SHA256 of the fixed label `obake-browser-v1`
under the current bearer. The server recomputes it and compares in
constant time; the bearer comparison is constant time too. Cookie
attributes: `Secure`, `HttpOnly`, `SameSite=Lax`, `Path=/api/v0`, and a
long `Max-Age`. The server ignores that age. GraphiQL and its
websocket are same-origin, so the browser sends the cookie on its own.

The TLS middleware treats three credentials differently:

- Unix socket: full access, no header required.
- `Authorization: Bearer` equal to the host bearer: full access.
- Browser cookie: GraphQL, subscriptions, file HTTP, and the login
  page only. Approve, rotate, and credential readout reject it.

**Why:** A phone never holds the bearer, so it cannot enroll another
device. There is no session table to lose on reboot. Rotation changes
the HMAC input and every cookie fails.

**Alternative:** A random per-browser token stored on disk. That allows
kicking one tablet, which this change does not need.

### 5. Device code in memory, cookie on poll

Unauthenticated routes, excluded from the bearer check:

- `GET /api/v0/login` shows the code.
- `POST /api/v0/login/start` returns a typed code and a poll secret.
- `POST /api/v0/login/poll` with that secret returns pending, or sets
  the cookie once approved.

`approveLogin(userCode)` is a GraphQL mutation. The typed code is eight
characters from an unambiguous alphabet, shown grouped, and valid for
10 minutes (inside the 15-minute spec bound). The poll secret is 256
bits and is what the page must present to receive the cookie. Pending
logins live in process memory, capped, with a limit on failed approve
attempts. A restart drops them. The granted cookie does not.

### 6. Rotate swaps the in-memory bearer

`auth rotate` generates a new UUID, writes the token file atomically
mode 0600, and replaces the bearer the running service checks. The
certificate files are not touched. The mutation returns the new bearer.
The CLI writes it into the selected host entry when that entry exists
and leaves the fingerprint as it is. Callers must be the Unix socket or
the previous bearer.

The bearer in `AppState` becomes something a request can re-read
(`RwLock` or equivalent) instead of a `String` copied at startup.

## Risks / Trade-offs

- [Phone still sees a self-signed warning] → The device code does not
  install trust. The operator accepts the warning once per browser, or
  the certificate stays wrong until the cert files are regenerated
  after a rename.
- [One shared cookie value] → Any copy of that cookie works until
  rotate. That matches a single kill switch. A stolen phone is cleared
  by `obake auth rotate`, which also forces every CLI to log in again.
- [Pending logins vanish on restart] → The browser shows a new code.
  Already issued cookies stay valid.
- [Hostname recorded at first start] → A later `hostnamectl` leaves the
  old names on the certificate. Remove the cert files, restart, and
  `auth login` again. Fingerprints change; bearers do not have to.
- [Stolen enrolled laptop holds the bearer] → It can call rotate.
  Recovery is `obake auth rotate` on the host socket, which needs no
  bearer, then login from each laptop.

## Migration Plan

- Deploy the service and CLI together. The first HTTPS start keeps the
  existing token file and adds the key and certificate beside it.
- Laptop configs that still say `http://` fail the fingerprint check
  until `obake auth login <ssh-user@host>`.
- Commands with no config file keep using the Unix socket when it
  exists. With neither a config file nor a socket, the CLI tells the
  operator to run `obake auth login <ssh-user@host>`.
- Rollback to a plaintext listener means current HTTPS clients will
  fail until their config endpoint is `http://` again and the
  fingerprint check is gone. The token file remains valid either way.
