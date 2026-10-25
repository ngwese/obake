# Spec Delta

## MODIFIED Requirements

### Requirement: PTP lock gates media nodes

The daemon SHALL create transmitting and receiving nodes only while
`ptp4l` reports a lock and a grandmaster identity, and only after that
lock has remained continuously true for the configured lock-hold
duration. Losing the lock SHALL remove those nodes and SHALL reset the
hold timer. A later continuous lock for the full hold duration SHALL
create them again when they still qualify. The daemon SHALL NOT step or
slew any clock.

#### Scenario: Losing the lock removes the stream nodes

- **WHEN** the PTP lock is lost while sources or sinks are active
- **THEN** those RTP nodes are removed

#### Scenario: Sources stay down during the lock hold

- **WHEN** `ptp4l` has just become locked and the configured lock-hold
  duration has not yet elapsed without an unlock
- **THEN** configured sources do not transmit
- **AND** configured sinks do not join a group

#### Scenario: Sources come up after a continuous hold

- **WHEN** `ptp4l` stays locked for the configured lock-hold duration
  and a source is otherwise legal
- **THEN** that source transmits

#### Scenario: Unlock during the hold restarts the timer

- **WHEN** the lock is lost before the hold duration elapses, then
  lock returns
- **THEN** the hold timer starts again from zero
- **AND** sources do not transmit until the full hold elapses again

### Requirement: Stream state can be queried

The `obake` user SHALL be able to query the running daemon for the PTP
lock state and, for each configured source and sink, whether it is
active and why it is held down. When the clock is locked but the
lock-hold has not elapsed, the query SHALL report that hold distinctly
from an unlocked clock.

#### Scenario: Status reports an unlocked source

- **WHEN** the PTP clock is unlocked and a source is configured
- **THEN** the query reports the clock unlocked and that source held
  down

#### Scenario: Status reports lock hold

- **WHEN** the PTP clock is locked, the lock-hold has not elapsed, and
  a source is configured
- **THEN** the query reports the clock locked and that source held
  down for the lock hold

## ADDED Requirements

### Requirement: Lock-hold duration is configurable

The daemon configuration SHALL accept a non-negative lock-hold duration
in milliseconds. When the key is absent, the daemon SHALL use a
non-zero default. A value of zero SHALL bring media nodes up as soon as
`ptp4l` reports locked with a grandmaster (no extra dwell).

#### Scenario: Zero hold matches immediate bring-up

- **WHEN** the lock-hold duration is 0 ms, the clock is locked with a
  grandmaster, and a source is otherwise legal
- **THEN** that source transmits without waiting for a dwell

#### Scenario: Absent key uses the default hold

- **WHEN** the configuration omits the lock-hold key
- **THEN** the daemon applies its default non-zero hold before sources
  may transmit
