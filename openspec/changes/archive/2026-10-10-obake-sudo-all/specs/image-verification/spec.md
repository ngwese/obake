# Spec Delta

## MODIFIED Requirements

### Requirement: Root lock and update sudo checked after boot

After a successful boot, the verification SHALL confirm as `obake` that empty-
password `su` to root fails (proving the root account is locked without reading
`/etc/shadow`) and that `obake` may invoke both `obake-update` and a general
root shell via passwordless sudo.

#### Scenario: Root password field locked

- **WHEN** the installed host has booted in QEMU and guest functional checks
  run as `obake`
- **THEN** the harness confirms `su` to root with an empty password fails

#### Scenario: obake-update sudo allowed, shell sudo denied

- **WHEN** the installed host has booted in QEMU and guest functional checks
  run as `obake`
- **THEN** the harness confirms `sudo -n obake-update status` succeeds and
  `sudo -n /bin/sh -c true` succeeds

### Requirement: Guest functional checks run as obake

Guest functional check suites installed into verification images SHALL run as
the `obake` login user, using only privilege paths available to that user
(passwordless sudo for all commands, and polkit for systemd unit and power
operations). They SHALL NOT run as uid 0 and SHALL NOT invoke raw
`rauc install` as root.

#### Scenario: Verify service identity

- **WHEN** a `VERIFY=1` image boots and guest functional checks start
- **THEN** the check process runs as user `obake`

#### Scenario: Update checks use the client

- **WHEN** guest functional checks exercise bundle install, reject, or
  rollback
- **THEN** those actions go through `obake-update` rather than a direct root
  `rauc install`
