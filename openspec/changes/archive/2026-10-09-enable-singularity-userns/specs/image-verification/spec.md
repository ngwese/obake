# Spec Delta

## ADDED Requirements

### Requirement: Guest functional checks run as obake

Guest functional check suites installed into verification images SHALL run as
the `obake` login user, using only privilege paths available to that user
(passwordless sudo for `/usr/bin/obake-update`, and polkit for systemd unit and
power operations). They SHALL NOT run as uid 0 and SHALL NOT invoke raw
`rauc install` as root.

#### Scenario: Verify service identity

- **WHEN** a `VERIFY=1` image boots and guest functional checks start
- **THEN** the check process runs as user `obake`

#### Scenario: Update checks use the client

- **WHEN** guest functional checks exercise bundle install, reject, or
  rollback
- **THEN** those actions go through `obake-update` rather than a direct root
  `rauc install`

### Requirement: Singularity run checked after boot

After a successful boot, the verification SHALL confirm that the `obake` user
can run a local Singularity SIF embedded in the verification image, so missing
user-namespace or FUSE support fails headless verification before release.

#### Scenario: Local SIF run as obake

- **WHEN** the installed host has booted in QEMU and guest functional checks
  run
- **THEN** the harness confirms `singularity run` (or equivalent) of the
  embedded local SIF succeeds as `obake`

## MODIFIED Requirements

### Requirement: Functional checks after boot

After a successful boot, the verification SHALL check core host behavior as
the `obake` login user, including the read-only root and the login-user
writable contract (`/home` writable; `/etc` and root writes fail), observe
`/etc` overlay topology without writing `/etc` as `obake`, and SHALL exercise
an update apply and revert through `obake-update`.

#### Scenario: Core host behavior checked

- **WHEN** the installed host has booted in QEMU
- **THEN** the harness confirms the root is read-only, the login-user
  writable contract behaves as specified, and an update applies and reverts
  through `obake-update` as specified

#### Scenario: Host user access checked

- **WHEN** the installed host has booted in QEMU
- **THEN** the harness confirms the host user can manage units and read the
  kernel log and full system journal

### Requirement: Root lock and update sudo checked after boot

After a successful boot, the verification SHALL confirm as `obake` that empty-
password `su` to root fails (proving the root account is locked without reading
`/etc/shadow`) and that `obake` may invoke `obake-update` via passwordless sudo
without being granted a general root shell.

#### Scenario: Root password field locked

- **WHEN** the installed host has booted in QEMU and guest functional checks
  run as `obake`
- **THEN** the harness confirms `su` to root with an empty password fails

#### Scenario: obake-update sudo allowed, shell sudo denied

- **WHEN** the installed host has booted in QEMU and guest functional checks
  run as `obake`
- **THEN** the harness confirms `sudo -n obake-update status` succeeds and
  `sudo -n /bin/sh -c true` is denied
