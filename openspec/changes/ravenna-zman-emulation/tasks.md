# Tasks

## 1. ZMAN listener and mock page

- [x] 1.1 Add a ZMAN HTTP listener in `services/obake-ravenna` bound to the
      PTP interface address, default port 9090, and verify a unit test on an
      ephemeral port serves `GET /advanced/index.html` as HTML that
      includes the host name
- [x] 1.2 Publish `_http._tcp` subtype `_merging_zman` on the PTP ifindex
      at that listen port, IPv4 only, and withdraw it when the interface
      is gone; verify Avahi recording tests cover publish and withdraw
      without dropping the existing RTSP node or session records
- [x] 1.3 Accept a WebSocket Bayeux handshake at `/cometd/handshake`:
      `/meta/handshake` returns success, a client id, and `websocket`,
      and `/meta/connect` keeps the session open. Verify a unit test
      completes that exchange and that no source or sink channel is
      published
- [x] 1.4 Keep port 9090 so the user unit needs no capability and
      `90-obake.conf` does not lower `ip_unprivileged_port_start`
- [x] 1.5 Document the listener, the mock URL, port 9090, and
      `/cometd/handshake` in `services/obake-ravenna/REFERENCE_GUIDE.md`;
      verify the guide names those values

## 2. Capture the Merging API

- [ ] 2.1 Record the Bayeux frames after `/cometd/handshake` from the
      Anubis advanced page (port 80) for listing sources, listing sinks,
      channel names, and connect. A VAD capture on port 9090 may be
      kept beside it for comparison. Save sanitized fixtures under
      `services/obake-ravenna` and note which device they came from.
      Verify each of those four operations has an Anubis fixture. Do not
      invent channel names

## 3. Enumerate configured streams

- [ ] 3.1 Implement the captured source-list and sink-list operations from
      `config.toml`, with generated channel names `<stream> <n>` when names
      are omitted; verify tests replay the fixtures and check an 8-channel
      source and one sink
- [ ] 3.2 Document those operations and the generated channel names in
      `services/obake-ravenna/REFERENCE_GUIDE.md`; verify the guide matches
      the fixture paths and the default name pattern

## 4. Connect a peer source to a sink

- [ ] 4.1 Feed browsed peer SDP into the discovery map the sink qualifier
      already reads, and implement the captured connect so one configured
      sink uses that peer; verify tests cover a successful connect and a
      rejected unknown sink that leaves other streams unchanged
- [ ] 4.2 Document the connect behavior in
      `services/obake-ravenna/REFERENCE_GUIDE.md`; verify the guide states
      that RTP still waits on the existing qualify rules
- [ ] 4.3 If a new crate was added, bump its pin in
      `services/obake-ravenna`; if none was added, leave pins unchanged.
      Verify `cargo test` in `services/obake-ravenna` passes headlessly
