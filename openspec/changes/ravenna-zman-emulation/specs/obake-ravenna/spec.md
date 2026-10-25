# Spec Delta

## ADDED Requirements

### Requirement: Advertises a ZMAN HTTP service

The daemon SHALL publish DNS-SD type `_http._tcp` with subtype
`_merging_zman` on the PTP interface, using the port it is listening
on. The instance name SHALL be the host name. Losing the PTP interface
SHALL withdraw the record. The existing RAVENNA RTSP node and session
records SHALL remain.

#### Scenario: The ZMAN service is announced with the listen port

- **WHEN** the PTP interface is present and the daemon is running
- **THEN** a `_merging_zman` HTTP service is published on that interface
- **AND** its port is the daemon's ZMAN listen port

#### Scenario: The RTSP records stay published

- **WHEN** the ZMAN HTTP service is published and a source is transmitting
- **THEN** the RAVENNA RTSP node and that source's session record are
  still published

### Requirement: Serves a ZMAN advanced page

The daemon SHALL answer `GET /advanced/index.html` on the ZMAN listener
with an HTML page that identifies the host. The page SHALL be available
before any Merging service channel is implemented.

#### Scenario: The mock page is reachable

- **WHEN** a client requests `/advanced/index.html` on the ZMAN listen
  port
- **THEN** the response is HTML
- **AND** the body names this host

### Requirement: Accepts a CometD handshake

The daemon SHALL accept a WebSocket Bayeux handshake at
`/cometd/handshake`. A `/meta/handshake` request SHALL be answered with
a successful handshake that includes a client id and `websocket` as a
supported connection type. Service channels that list streams or connect
them SHALL NOT be implemented until a capture names them.

#### Scenario: Handshake succeeds

- **WHEN** a client opens `ws://<listen>/cometd/handshake` and sends a
  Bayeux `/meta/handshake` message
- **THEN** the reply is a successful handshake
- **AND** the reply includes a client id and the `websocket` connection
  type

#### Scenario: Unknown service channels are not invented

- **WHEN** the handshake has succeeded and no capture yet names a
  service channel
- **THEN** the daemon does not publish source or sink data on a guessed
  channel

### Requirement: Lists configured streams and channel names

Once the captured ZMAN read API is implemented, the daemon SHALL list
every configured source and every configured sink through that API.
Each channel SHALL have a display name. When the configuration omits a
name, the daemon SHALL use the stream name plus a 1-based channel index.

#### Scenario: A configured source lists one name per channel

- **WHEN** a source is configured with 8 channels and no channel names
- **AND** a client performs the captured source-list request
- **THEN** the response includes that source
- **AND** it includes eight channel names derived from the source name

#### Scenario: A configured sink is listed

- **WHEN** a sink is configured
- **AND** a client performs the captured sink-list request
- **THEN** the response includes that sink

### Requirement: Attaches a peer source to a configured sink

Once the captured connect API is implemented, a connect request SHALL
select one discovered peer source and one configured sink. The daemon
SHALL apply that peer's session description to the sink so the sink can
qualify. A connect for an unknown sink or an unknown peer source SHALL
be rejected and SHALL NOT change other streams.

#### Scenario: Connecting a discovered source brings the sink up

- **WHEN** a peer source has been discovered and a sink is configured
- **AND** a client performs the captured connect of that peer to that sink
- **THEN** the sink uses that peer's session description
- **AND** the sink comes up when it otherwise qualifies

#### Scenario: An unknown target is rejected

- **WHEN** a connect request names a sink that is not configured
- **THEN** the daemon rejects the request
- **AND** no configured stream changes
