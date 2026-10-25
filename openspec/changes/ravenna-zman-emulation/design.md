# Design

## Context

See proposal.md for motivation. The daemon already publishes
`_ravenna._sub._rtsp._tcp` and per-source `_ravenna_session` records
through Avahi, and it serves RTSP on port 8554. The control loop's
discovery map is currently empty, so configured sinks do not yet
qualify from browsed peers. The user unit
`image/os/buildroot/package/obake-ravenna/obake-ravenna.service` runs
as the `obake` user and has no `CAP_NET_BIND_SERVICE`.

Merging documents that a ZMAN/Anubis device is `_http._tcp` subtype
`_merging_zman`, that the advanced UI is `/advanced/index.html`, and
that ANEMAN and MAD/VAD open that service. Both the CoreAudio VAD advanced page (`ws://<host>:9090/cometd/handshake`)
and the Anubis advanced page (`ws://<anubis>/cometd/handshake`, port 80)
open a Bayeux/CometD handshake on that path. The port is the product
class: 9090 for VAD/MAD/MassCore, 80 for ZMAN/Anubis. The service-channel
names after the handshake are still uncaptured.

## Goals / Non-Goals

**Goals:**

- One daemon process owns the ZMAN listener and the existing RTSP path.
- A mock advanced page is reachable, and a Bayeux handshake succeeds,
  before any stream service channel exists.
- Later slices follow a captured exchange, stored as fixtures.
- Configured `[[source]]` / `[[sink]]` entries are what the read API
  lists. Channel names default when unset.
- A captured connect updates one sink's peer target.

**Non-Goals:**

- Replacing RTSP, SDP, or PTP lock-hold.
- Emulating Anubis missions, monitors, or the mixer.
- Guessing URL paths or JSON fields that are not in the capture.
- NMOS IS-04/IS-05.

## Decisions

### Phased build

1. Plumbing: HTTP listener + mDNS + static `/advanced/index.html` +
   WebSocket Bayeux handshake at `/cometd/handshake`.
2. Capture: record the Bayeux frames after handshake (subscribe and
   service channels) for list-sources, list-sinks, channel names, and
   connect. Check sanitized fixtures into `services/obake-ravenna`.
3. Read API: implement only the captured service channels from `config.toml`.
4. Connect: implement only the captured channel, and feed the chosen
   peer SDP into the sink qualify path (the discovery map is empty today).

Stop at the end of a phase if the capture does not contain the next
operation. Do not invent the next route to keep the task list moving.

### Listen port

Default the listener to **9090** (`ws://<host>:9090/cometd/handshake`).
Publish that port in the SRV record. The mDNS subtype stays
`_merging_zman`, so the record is still a ZMAN-class device even though
9090 is the port VAD uses. Port 9090 needs no capability and no sysctl
change. `obake-ravenna run --no-zman` skips the listener and the
`_merging_zman` record. Unit tests bind an ephemeral port.

Bind the PTP interface address, same as the RTSP listener's scope.

### Advertisement

Add a publish kind beside the RTSP node/session kinds. Type `_http._tcp`,
subtype `_merging_zman._sub._http._tcp`, instance name from the same
host-name source as the RTSP node, IPv4 only, on the PTP ifindex.

### Mock page and handshake

Static HTML in the binary. It states the hostname. The same listener
accepts a WebSocket at `/cometd/handshake` and answers a Bayeux
`/meta/handshake` with `successful: true`, a generated `clientId`, and
`supportedConnectionTypes` including `websocket`. It also answers
`/meta/connect` well enough to keep the socket open. It does not
subscribe to or publish guessed service channels. The next capture is
the subscribe frames the page sends after this handshake.

### Channel names

Do not require new TOML keys for the first read slice. Generate
`"<stream name> <n>"` with `n` starting at 1. Optional explicit names
can be added if the capture shows the client requires stable custom
labels; that is a config addition, not a second discovery.

### Connect

The connect body, once captured, selects a configured sink and a peer
source. Persist the selection in the daemon's runtime state (not
necessarily `config.toml` in the first cut) and pass that SDP into
qualify. Unknown sink or unknown peer returns an error and leaves other
streams alone. RTP still starts only when qualify already allows it
(lock hold, rate, domain).

### Alternatives

- NMOS instead of ZMAN HTTP: documented, but MAD/VAD panels will not
  use it. Rejected for this goal.
- Scrape or fake the advanced HTML only: gets a browser page, not a
  matrix. The mock page is only the first slice.
- Listen on 80 like Anubis: needs a privileged port. Rejected in favor
  of 9090 with the `_merging_zman` subtype kept.

## Risks / Trade-offs

- [Bind failure] → log it and keep running. RTSP does not depend on the
  ZMAN listener.
- [VAD and Anubis CometD channels differ] → fixtures must say which device
  they came from; do not replay a VAD subscribe set against a ZMAN
  subtype until one capture of each is compared.
- [Handshake needs Anubis-only objects] → capture will show it; stop
  and extend the fixture rather than stubbing unrelated modules.
- [Browse stays empty] → connect cannot resolve a peer until discovery
  results are stored. That wiring is part of the connect slice.
- [Schema drifts across firmware] → fixtures name the device and build
  they were taken from; do not generalize past them.

## Migration Plan

- New listener and mDNS record appear when the daemon with this change
  is installed. Existing configs keep working; RTSP behavior is unchanged.
- Rollback is the previous daemon binary. No config key is required for
  the mock page.

## Open Questions

- None that change the phase order. The port-80 decision waits on a
  live client check during the capture slice.
