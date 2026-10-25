# Spec Delta

## ADDED Requirements

### Requirement: Multicast allocation mode selects port or merging layout

The daemon multicast settings SHALL accept an allocation mode of `port`
or `merging`. When the mode is omitted, the daemon SHALL use `port`.
Mode `port` is the existing shared-group, stepped-port layout. Mode
`merging` mimics observed Merging device behavior: a shared even RTP
port and a distinct group per source. A generic address-stepping mode
SHALL NOT be introduced by this change.

#### Scenario: Omitted allocation keeps port mode

- **WHEN** `[multicast]` omits `allocation`
- **AND** two sources transmit without pinned addresses or ports
- **THEN** both use the configured base multicast address
- **AND** their RTP ports are `base_port` and `base_port + 2`

#### Scenario: Unknown allocation is rejected

- **WHEN** `allocation` is set to a value other than `port` or `merging`
- **THEN** the configuration fails to load

### Requirement: Merging allocation derives the group from PTP unicast

When allocation is `merging` and a source does not pin an address, the
daemon SHALL set that source's multicast group to
`239.{session}.{o3}.{o4}`, where `{o3}` and `{o4}` are the third and
fourth octets of the PTP interface IPv4 address, and `{session}` is
`session_base + index` with `index` the source's zero-based position
among configured sources. `session_base` SHALL default to `1`. The
daemon SHALL use the configured even `base_port` for every such source
that does not pin a port. The daemon SHALL NOT require
`[multicast].address` in this mode.

#### Scenario: Two sources follow the host unicast

- **WHEN** allocation is `merging`, `session_base` is `1`, and
  `base_port` is `5004`
- **AND** the PTP interface address is `192.168.60.50`
- **AND** the first two sources have no pinned address or port
- **THEN** their destinations are `239.1.60.50:5004` and
  `239.2.60.50:5004`

#### Scenario: Missing PTP IPv4 holds unpinned merging sources

- **WHEN** allocation is `merging`
- **AND** the PTP interface has no IPv4 address
- **AND** a source does not pin an address
- **THEN** that source does not transmit

#### Scenario: Session octet overflow holds the source

- **WHEN** allocation is `merging`
- **AND** `session_base + index` does not fit in one octet
- **THEN** that source does not transmit

#### Scenario: Merging sources share the base port

- **WHEN** allocation is `merging`, `base_port` is 5004, and the first
  two sources transmit without pinned ports
- **THEN** both use RTP port 5004

#### Scenario: A pin overrides merging allocation

- **WHEN** allocation is `merging` and the PTP interface address is
  `192.168.60.50`
- **AND** the first source pins `address = "239.9.60.50"` and omits
  `port`
- **THEN** that source transmits to `239.9.60.50` at `base_port`

### Requirement: Merging mode rejects a configured base address

When allocation is `merging`, the daemon SHALL reject a configuration
that sets `[multicast].address`. Per-source pinned `address` values
remain allowed.

#### Scenario: Base address with merging fails to load

- **WHEN** `allocation` is `merging` and `[multicast].address` is set
- **THEN** the configuration fails to load

## MODIFIED Requirements

### Requirement: Transmit address is one scoped multicast base

Under allocation `port`, the transmit multicast address SHALL come from
the daemon multicast settings. The daemon SHALL transmit only when that
address is inside `239.0.0.0/8` and is not `239.255.255.255`. Under
allocation `port`, the daemon SHALL NOT assign a different address to
each source by incrementing the base. Under allocation `merging`, the
group for an unpinned source SHALL follow the merging allocation
requirement instead of a shared base address.

#### Scenario: The SAP group stays down

- **WHEN** allocation is `port` and the multicast base address is
  `239.255.255.255`
- **THEN** no source transmits

#### Scenario: Sources share the base address

- **WHEN** allocation is `port` (or omitted)
- **AND** two sources are transmitting and neither sets its own
  address
- **THEN** both use the configured base multicast address

### Requirement: Transmit ports step from an even base

Under allocation `port`, the daemon SHALL assign each source an even
RTP port of `base_port + 2 * index`, where `index` is that source's
position among configured sources starting at zero. Under allocation
`merging`, every unpinned source SHALL use `base_port` (no port step).
`base_port` SHALL be even. The daemon SHALL NOT send that source's RTP
to the following odd port.

#### Scenario: The first two sources use consecutive even ports

- **WHEN** allocation is `port` (or omitted), `base_port` is 5004, and
  the first two sources are transmitting
- **THEN** their RTP ports are 5004 and 5006

#### Scenario: An odd base port stays down

- **WHEN** `base_port` is odd
- **THEN** no source transmits

### Requirement: A source may pin address or port

A source MAY set its own multicast address or port under either
allocation mode. A pinned port SHALL be even. Two sources SHALL NOT
transmit to the same address and port.

#### Scenario: A duplicate destination stays down

- **WHEN** two sources resolve to the same multicast address and port
- **THEN** neither source transmits
