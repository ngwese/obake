# Spec Delta: obake-session

## MODIFIED Requirements

### Requirement: The CLI uses noun-verb commands

`obake` subcommands SHALL follow `<noun> <verb>` by default. Nouns SHALL
be schema entities. Verbs SHALL include `ls`, `start`, `stop`, `create`,
`delete`, `get`, and `describe`. A plural noun SHALL NOT be a top-level
command; listing SHALL use `<noun> ls`. Documented exceptions MAY omit
the pattern when a flat verb would be unclear. The noun `auth` is a
documented exception and is not a schema entity.

#### Scenario: Listing uses noun ls

- **WHEN** the operator lists installed rigs
- **THEN** the command is `obake rig ls` and not a plural top-level
  `rigs` command

#### Scenario: Session describe defaults to the active session

- **WHEN** a session is active and the operator runs
  `obake session describe` with no id
- **THEN** the output describes that active session

#### Scenario: File commands default to the active session

- **WHEN** a session is active and the operator runs `obake file ls`
  without `--session`
- **THEN** the listed files are that session's work area

#### Scenario: File get writes like cp

- **WHEN** the operator runs `obake file get take.wav .` for the active
  session
- **THEN** the client performs an HTTP GET of the raw bytes and writes a
  local file named `take.wav`

#### Scenario: File get can stream to stdout

- **WHEN** the operator runs `obake file get take.wav -`
- **THEN** the raw bytes are written to stdout

#### Scenario: File put uploads bytes

- **WHEN** the operator runs `obake file put ./local.ck remote.ck`
- **THEN** the client performs an HTTP PUT of the local file bytes into
  the session work area at `remote.ck`

#### Scenario: File commands accept --session

- **WHEN** the operator passes `--session <id>` to `file ls`, `file get`,
  or `file put`
- **THEN** the operation targets that session's work area instead of the
  active session

#### Scenario: Auth is an allowed noun

- **WHEN** the operator enrolls, approves a browser, or rotates the
  bearer
- **THEN** the commands are `obake auth login`, `obake auth approve`,
  and `obake auth rotate`

### Requirement: The client config names hosts and a default

The `obake` client config SHALL list hosts, each with an endpoint, a
bearer token, and a certificate fingerprint, and SHALL name the default
host. The client SHALL use that default unless a command names another
host. A command-line endpoint or token SHALL replace the chosen host's
value. A missing file SHALL leave the local Unix socket in use when
that socket exists. When the config file and the local socket are both
absent, the command SHALL fail without sending a request and SHALL tell
the operator to run `obake auth login <ssh-user@host>`. An
unknown host name SHALL fail and SHALL NOT send a request. An HTTPS
request SHALL fail when the peer certificate fingerprint does not match
the stored fingerprint, or when the fingerprint is missing, and that
failure SHALL NOT perform a GraphQL operation or file transfer.

#### Scenario: The default host is used

- **WHEN** the config names hosts `studio` and `spare`, sets the default
  to `studio`, and the operator runs `obake` without naming a host
- **THEN** the request goes to `studio`'s endpoint with `studio`'s token

#### Scenario: A command selects another host

- **WHEN** that config's default is `studio` and the operator names
  `spare`
- **THEN** the request goes to `spare`'s endpoint with `spare`'s token

#### Scenario: A flag replaces the chosen host

- **WHEN** the chosen host has an endpoint and the operator passes a
  different `--endpoint`
- **THEN** the request goes to the flag's endpoint

#### Scenario: A missing file uses the local socket

- **WHEN** no config file exists, the operator passes no endpoint, and
  the local Unix socket exists
- **THEN** the client uses the local Unix socket

#### Scenario: A missing file with no local socket explains login

- **WHEN** no config file exists, the operator passes no endpoint, and
  the local Unix socket does not exist
- **THEN** the command fails without sending a request and tells the
  operator to run `obake auth login <ssh-user@host>`

#### Scenario: An unknown host is rejected

- **WHEN** the operator names a host that is not in the config
- **THEN** the command fails and no request is sent

#### Scenario: A mismatched certificate is rejected

- **WHEN** the chosen host's stored fingerprint differs from the
  certificate presented by its endpoint
- **THEN** the command fails and no GraphQL operation or file transfer
  is performed

#### Scenario: A missing fingerprint is rejected

- **WHEN** the chosen host's endpoint is HTTPS and the config has no
  fingerprint for that host
- **THEN** the command fails and no GraphQL operation or file transfer
  is performed

### Requirement: GraphQL and file HTTP are served locally and on the network

The service SHALL accept GraphQL and session-file HTTP on a Unix socket
in the `obake` user's runtime directory and on a configured TLS address.
A request to the TLS address SHALL present either the host's bearer
token or a valid browser session cookie. A TLS request with neither
SHALL be rejected and SHALL NOT change the session or work-area files.
A request on the Unix socket SHALL NOT require the token. The TLS
address SHALL NOT accept plaintext HTTP.

#### Scenario: A tokened network client is served

- **WHEN** a client queries the configured TLS address with the bearer
  token
- **THEN** the query result is returned

#### Scenario: A network request without the token is rejected

- **WHEN** a client queries the configured TLS address without the
  bearer token and without a browser session cookie
- **THEN** the request is rejected and the active session is unchanged

#### Scenario: The local socket does not require the token

- **WHEN** a client queries the user Unix socket without a bearer token
- **THEN** the query result is returned

#### Scenario: Network file GET requires the token

- **WHEN** a client GETs a session file on the TLS address without the
  bearer token and without a browser session cookie
- **THEN** the request is rejected and no file bytes are returned

#### Scenario: A browser cookie is served

- **WHEN** a client queries the configured TLS address with a valid
  browser session cookie and without the bearer token
- **THEN** the query result is returned

#### Scenario: Plaintext HTTP on the TLS address is rejected

- **WHEN** a client sends plaintext HTTP to the configured TLS address
- **THEN** the request is rejected and the active session is unchanged

## ADDED Requirements

### Requirement: The host serves a stable self-signed certificate

The service SHALL present a self-signed certificate on its TLS
listener. It SHALL create the certificate and private key when they
are absent, and SHALL present the same certificate after a process
restart or a host boot. The certificate SHALL name the system hostname
and that hostname with a `.local` suffix.

#### Scenario: First start creates a certificate

- **WHEN** the service starts and no certificate is stored yet
- **THEN** it creates one and the TLS listener presents it

#### Scenario: Restart keeps the same certificate

- **WHEN** the service starts again after a process restart
- **THEN** the TLS listener presents the same certificate as before

#### Scenario: The certificate names the host

- **WHEN** the system hostname is `studio`
- **THEN** the certificate includes `studio` and `studio.local`

### Requirement: The host credential is local to the socket

`obake auth credential` SHALL print the current bearer, the TLS listen
port, and the certificate fingerprint when it can reach the Unix
socket. The TLS listener SHALL NOT return the bearer or the private
key, including to a client that sends the bearer or a browser cookie.

#### Scenario: The local socket prints the credential

- **WHEN** the operator runs `obake auth credential` on the host
- **THEN** the output includes the bearer, the listen port, and the
  certificate fingerprint

#### Scenario: The network cannot read the credential

- **WHEN** a client on the TLS listener requests the credential with
  the bearer or with a browser cookie
- **THEN** the bearer and the private key are not returned

### Requirement: SSH login stores the host credential

`obake auth login <ssh-user@host>` SHALL use the system SSH client to
read that host's bearer, listen port, and certificate fingerprint.
Success SHALL record an `https` endpoint, the bearer, and the
fingerprint under the host part of the argument, and SHALL set the
default host when none is set. A failed SSH SHALL leave the config
unchanged. Logging in again for one host SHALL leave every other host
unchanged.

#### Scenario: Login writes a usable host entry

- **WHEN** the operator runs `obake auth login obake@studio.local` and
  SSH succeeds
- **THEN** the config contains host `studio.local` with an `https`
  endpoint, the bearer, and the fingerprint
- **AND** a following command to that host is accepted by the service

#### Scenario: Login over a remote command still enrolls

- **WHEN** login reaches the host with a non-interactive SSH command
- **THEN** the config still receives the bearer and the fingerprint

#### Scenario: A failed SSH does not change config

- **WHEN** SSH to the named user and host fails
- **THEN** the client config file is unchanged

#### Scenario: A second login updates only that host

- **WHEN** the config has hosts `studio.local` and `spare.local` and
  login succeeds again for `obake@studio.local`
- **THEN** `studio.local` has the current bearer and fingerprint
- **AND** `spare.local` is unchanged

### Requirement: A device code admits one browser

The TLS login page SHALL show a device code to a browser that has no
session. `obake auth approve <code>` SHALL accept a current code from
the Unix socket or from a client sending the host bearer. The browser
that started that login SHALL then receive a session cookie. A client
that did not start the login SHALL NOT receive a cookie by presenting
the code alone. An unknown or expired code SHALL be rejected and SHALL
NOT set a cookie.

#### Scenario: Approve on the host signs the waiting browser in

- **WHEN** a browser has been shown a device code and the operator runs
  `obake auth approve` with that code on the host
- **THEN** that browser receives a session cookie that authorizes a
  GraphQL query

#### Scenario: Approve from an enrolled CLI signs the browser in

- **WHEN** a laptop CLI holds the host bearer and the operator runs
  `obake auth approve` with the browser's code
- **THEN** that browser receives a session cookie

#### Scenario: The typed code alone does not yield a cookie

- **WHEN** a second client presents a device code it did not start
- **THEN** it receives no session cookie

#### Scenario: An expired code is rejected

- **WHEN** the operator approves a code more than 15 minutes after the
  browser was shown it
- **THEN** approval fails and the browser receives no session cookie

#### Scenario: A restart drops a pending code

- **WHEN** the service restarts after a code is shown and before it is
  approved
- **THEN** approving that code fails

### Requirement: The browser cookie lasts until the bearer changes

A browser session cookie SHALL authorize GraphQL, its subscription
socket, and session-file HTTP on the TLS listener for as long as the
host bearer is unchanged, including after a service restart. The
cookie SHALL NOT authorize approve, rotate, or reading the host
credential.

#### Scenario: The cookie survives a service restart

- **WHEN** a browser holds a session cookie and the service restarts
  without a bearer change
- **THEN** that cookie still authorizes a GraphQL query

#### Scenario: The cookie cannot approve another browser

- **WHEN** a client presents only a valid browser cookie and attempts
  to approve a device code
- **THEN** approval fails and the bearer is unchanged

### Requirement: Rotating the bearer revokes other clients

`obake auth rotate` SHALL replace the host bearer in the running
service when called on the Unix socket or with the current bearer. It
SHALL leave the TLS certificate unchanged. The CLI SHALL write the new
bearer into its selected host entry when that entry exists. Cookies
issued before the rotation, and any other copy of the previous bearer,
SHALL be rejected. Rotate presented with only a browser cookie SHALL
fail and SHALL leave the bearer unchanged.

#### Scenario: Rotate on the host invalidates the old bearer

- **WHEN** the operator runs `obake auth rotate` on the host
- **THEN** the previous bearer is rejected, the new bearer is accepted,
  and the certificate fingerprint is unchanged

#### Scenario: Rotate from a laptop updates that laptop

- **WHEN** an enrolled CLI runs `obake auth rotate` using its stored
  bearer
- **THEN** that host's config entry holds the new bearer and the same
  fingerprint
- **AND** the previous bearer is rejected

#### Scenario: Rotate signs browsers out

- **WHEN** a browser holds a session cookie and the bearer is rotated
- **THEN** that cookie is rejected

#### Scenario: A cookie cannot rotate the bearer

- **WHEN** a client presents only a valid browser cookie and attempts
  to rotate
- **THEN** rotation fails and the current bearer is still accepted
