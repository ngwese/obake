# Spec Delta

## MODIFIED Requirements

### Requirement: PTP lock gates media nodes

The daemon SHALL create transmitting and receiving nodes only while
`ptp4l` reports a lock and a grandmaster identity, the absolute PTP
offset is within the configured offset threshold (when that threshold
is non-zero), and the lock has remained continuously true for the
configured lock-hold duration. Losing the lock SHALL remove those nodes
and SHALL reset the hold timer. A later continuous lock for the full
hold duration with offset within threshold SHALL create them again when
they still qualify. The daemon SHALL NOT step or slew any clock.

#### Scenario: Losing the lock removes the stream nodes

- **WHEN** the PTP lock is lost while sources or sinks are active
- **THEN** those RTP nodes are removed

#### Scenario: Sources stay down during the lock hold

- **WHEN** `ptp4l` has just become locked and the configured lock-hold
  duration has not yet elapsed without an unlock
- **THEN** configured sources do not transmit
- **AND** configured sinks do not join a group

#### Scenario: Sources come up after a continuous hold

- **WHEN** `ptp4l` stays locked for the configured lock-hold duration,
  the absolute PTP offset is within the configured threshold (or the
  threshold is zero), and a source is otherwise legal
- **THEN** that source transmits

#### Scenario: Sources stay down while offset is high

- **WHEN** `ptp4l` is locked with a grandmaster, the lock-hold has
  elapsed, the offset threshold is non-zero, and the absolute PTP
  offset exceeds that threshold
- **THEN** configured sources do not transmit
- **AND** configured sinks do not join a group

#### Scenario: Sources come up after hold and offset settle

- **WHEN** `ptp4l` stays locked for the configured lock-hold duration,
  the absolute PTP offset is within the configured threshold, and a
  source is otherwise legal
- **THEN** that source transmits

#### Scenario: Unlock during the hold restarts the timer

- **WHEN** the lock is lost before the hold duration elapses, then
  lock returns
- **THEN** the hold timer starts again from zero
- **AND** sources do not transmit until the full hold elapses again

### Requirement: Reads PTP state without spawning a process

The daemon SHALL read `ptp4l` lock state, domain, grandmaster clock
identity, and PTP offset over the read-only management Unix socket in
its own process. It SHALL NOT spawn `pmc` or any other helper for that
read. A missing socket SHALL be treated as unlocked. A failed offset
read while otherwise locked SHALL be treated as offset outside the
threshold when the threshold is non-zero.

#### Scenario: Lock is read with no child process

- **WHEN** the daemon queries the PTP clock
- **THEN** it uses the read-only management socket itself
- **AND** it does not create a child process for that query

#### Scenario: Offset is read with no child process

- **WHEN** the daemon queries the PTP offset
- **THEN** it uses the read-only management socket itself
- **AND** it does not create a child process for that query

### Requirement: Stream state can be queried

The `obake` user SHALL be able to query the running daemon for the PTP
lock state and, for each configured source and sink, whether it is
active and why it is held down. When the clock is locked but the
lock-hold has not elapsed, the query SHALL report that hold distinctly
from an unlocked clock. When the clock is locked and the hold has
elapsed but the absolute PTP offset exceeds the configured threshold,
the query SHALL report that offset hold distinctly from unlock and
from lock-hold.

#### Scenario: Status reports an unlocked source

- **WHEN** the PTP clock is unlocked and a source is configured
- **THEN** the query reports the clock unlocked and that source held
  down

#### Scenario: Status reports lock hold

- **WHEN** the PTP clock is locked, the lock-hold has not elapsed, and
  a source is configured
- **THEN** the query reports the clock locked and that source held
  down for the lock hold

#### Scenario: Status reports offset hold

- **WHEN** the PTP clock is locked, the lock-hold has elapsed, the
  absolute PTP offset exceeds the configured threshold, and a source
  is configured
- **THEN** the query reports the clock locked and that source held
  down for the offset hold

## ADDED Requirements

### Requirement: Offset threshold is configurable

The daemon configuration SHALL accept a non-negative maximum absolute
PTP offset in nanoseconds. When the key is absent, the daemon SHALL
use a non-zero default. A value of zero SHALL disable the offset gate
so media nodes may come up once lock and lock-hold (and other existing
checks) pass.

#### Scenario: Zero threshold disables offset gating

- **WHEN** the offset threshold is 0 ns, the clock is locked with a
  grandmaster, the lock-hold has elapsed, and a source is otherwise
  legal
- **THEN** that source transmits regardless of reported PTP offset

#### Scenario: Absent key uses the default threshold

- **WHEN** the configuration omits the offset-threshold key
- **THEN** the daemon applies its default non-zero threshold before
  sources may transmit
