# Spec Delta

## ADDED Requirements

### Requirement: Network time sync checked after boot

After a successful boot, the verification SHALL confirm that
`systemd-timesyncd` is enabled and active, so a disabled or missing NTP
service fails headless verification before release. The check SHALL NOT
require that the clock has already synchronized (sandboxes may block NTP).

#### Scenario: Guest sees timesyncd enabled and active

- **WHEN** the installed host has booted in QEMU and guest functional checks
  run
- **THEN** the harness confirms `systemd-timesyncd.service` is enabled and
  active
