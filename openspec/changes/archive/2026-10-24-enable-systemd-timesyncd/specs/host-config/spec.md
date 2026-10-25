# Spec Delta

## ADDED Requirements

### Requirement: Network time synchronization

The image SHALL enable `systemd-timesyncd` so that, when network connectivity
and NTP servers are reachable, the system clock is synchronized from the
network. Boot success SHALL NOT require a successful time sync.

#### Scenario: timesyncd enabled on a fresh image

- **WHEN** a host boots an unmodified image
- **THEN** `systemd-timesyncd.service` is enabled

#### Scenario: timesyncd runs after boot

- **WHEN** a host reaches multi-user after boot
- **THEN** `systemd-timesyncd.service` is active

#### Scenario: Boot does not wait for NTP

- **WHEN** the host boots without reachable NTP servers
- **THEN** boot still completes successfully without waiting on time sync
