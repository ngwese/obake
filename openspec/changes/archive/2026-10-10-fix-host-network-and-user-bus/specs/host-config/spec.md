# Spec Delta

## ADDED Requirements

### Requirement: Optional network cabling

Wired network interfaces managed by the host SHALL NOT be required to reach an
online state before boot is considered successful. With no cable attached,
`systemd-networkd-wait-online` SHALL NOT leave the system in a failed or
degraded state solely because those interfaces lack carrier.

#### Scenario: Uncabled boot does not fail wait-online

- **WHEN** the host boots with no wired interface cabled
- **THEN** `systemd-networkd-wait-online.service` does not fail for lack of
  carrier on managed wired links

#### Scenario: DHCP still applies when a cable is present

- **WHEN** a managed wired interface gains carrier
- **THEN** that interface still obtains addressing via DHCP as configured

### Requirement: SSH login reaches the obake user bus

An interactive SSH login as `obake` SHALL provide the environment variables
needed to locate the lingering user runtime and session bus, so
`systemctl --user` works without manually exporting those variables. The image
SHALL NOT require enabling PAM solely for this behaviour.

#### Scenario: systemctl --user works over SSH

- **WHEN** the `obake` user logs in over SSH after boot (with linger and the
  user manager already active)
- **THEN** `systemctl --user status` succeeds without the operator exporting
  `XDG_RUNTIME_DIR` or `DBUS_SESSION_BUS_ADDRESS`
