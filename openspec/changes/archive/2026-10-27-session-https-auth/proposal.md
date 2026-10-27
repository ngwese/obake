# Proposal

## Why

The session API on the network is plaintext HTTP guarded by a bearer
that the operator must copy by hand. Phones and tablets cannot present
that header, and a laptop CLI has no way to learn the token except by
reading a file. The host is headless, reached over SSH, and has no
external identity provider. SSH is already the operator's credential;
the network API should enroll from that and then serve HTTPS.

## What Changes

- **BREAKING**: The configured TCP listener serves HTTPS with a
  self-signed certificate the host generates. Plaintext HTTP on that
  listener is no longer accepted. The Unix socket stays unauthenticated.
- The client config stores each host's endpoint, bearer token, and
  certificate fingerprint. An HTTPS request whose peer certificate does
  not match the stored fingerprint fails. A missing fingerprint on an
  HTTPS host fails the same way.
- `obake auth login <ssh-user@host>` uses the local `ssh` client. A
  successful login writes that host's bearer and certificate
  fingerprint into this machine's client config.
- A browser that opens the session URL is shown a short device code.
  `obake auth approve <code>` accepts that code from a CLI on the host
  or from a CLI that already holds the bearer. The browser then keeps a
  long-lived cookie and can use GraphiQL without a pasted header.
- **BREAKING**: `obake auth rotate` replaces the host bearer in the
  running service. Every browser cookie and every other CLI's copy of
  the bearer stops working. The TLS certificate is unchanged, so
  enrolled clients keep the same fingerprint and log in again for a new
  token.
- The browser cookie is valid only for the current host bearer, including
  across reboots. It cannot approve further browsers or act as the CLI
  bearer.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `obake-session`: The network listener requires TLS and accepts either
  the host bearer or a browser cookie bound to that bearer. The client
  config gains a certificate fingerprint. New operator commands enroll
  a CLI over SSH, approve a browser device code, and rotate the bearer.

## Impact

- `services/obake-session`: TLS listener, certificate files, browser
  login page, approve and rotate operations, auth that accepts the
  bearer or the cookie.
- `services/obake`: HTTPS client that checks the pinned fingerprint,
  `auth` commands, client config fields. The host image already
  installs `/usr/bin/obake` beside `obake-session`.
- Persistent host state: the bearer, private key, and certificate live
  under the `obake` user's config on the user partition, so an OTA
  slot switch keeps the same identity and enrolled clients stay valid.
- Client config on each operator machine (`~/.config/obake/config.yaml`)
  gains a fingerprint. Existing remote entries that name `http://` stop
  working until `obake auth login`.
- Rig, session, and shape documents are unchanged. The bearer and
  certificate are per host and are not part of moving a rig to another
  host.
- No new external identity provider, account database, or public CA.
