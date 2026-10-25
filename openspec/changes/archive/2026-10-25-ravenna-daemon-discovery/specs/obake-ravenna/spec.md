# Spec Delta

## ADDED Requirements

### Requirement: The daemon discovers peer session SDP

The daemon SHALL browse RAVENNA device and session advertisements on the
`ptp4l` interface and SHALL retrieve session SDP over RTSP DESCRIBE for
configured sinks that are not yet established. It SHALL key each result
by the browsed device instance name and session instance name. A
configured sink whose `device` and `stream` match a discovered pair
SHALL use that SDP for qualify (subject to existing multicast, rate,
domain, and PTP gates). DESCRIBE paths SHALL use the session TXT `path`
when present, otherwise `/by-name/<session>`, with path segments
percent-encoded on the request line.

#### Scenario: A matching sink leaves device-absent

- **WHEN** a peer advertises device `Anubis_610267` and session
  `Anubis_610267_Combo 1/2` with acceptable SDP, PTP gates pass, and a
  sink is configured with that device and stream
- **THEN** that sink is not held down for `device-absent`

#### Scenario: Unknown device stays device-absent

- **WHEN** a sink names a device and stream that are not present in the
  discovery results
- **THEN** that sink remains held down for `device-absent`

#### Scenario: Session names with spaces DESCRIBE successfully

- **WHEN** a discovered session instance name contains spaces or
  parentheses and has no TXT `path`
- **THEN** the daemon DESCRIBEs `/by-name/<that name>` with a
  percent-encoded request-URI
- **AND** a successful response is available for sink qualify

### Requirement: Established sinks keep cached SDP

While a sink remains up, the daemon SHALL continue to use the SDP
already obtained for that `(device, stream)` and SHALL NOT re-DESCRIBE
or otherwise re-fetch that session's SDP on each control-loop tick or
discovery refresh. The daemon MAY DESCRIBE again after the sink goes
down, is removed from configuration, or the cached entry is cleared
because the peer advertisement disappeared.

#### Scenario: No DESCRIBE while the sink stays up

- **WHEN** a sink has qualified and is up from a prior DESCRIBE
- **THEN** subsequent control-loop ticks do not DESCRIBE that session
  again while the sink remains up

#### Scenario: DESCRIBE may run again after the sink drops

- **WHEN** a previously up sink becomes held down or is removed from
  configuration, and later a matching peer session is needed again
- **THEN** the daemon may DESCRIBE that session to re-establish SDP
