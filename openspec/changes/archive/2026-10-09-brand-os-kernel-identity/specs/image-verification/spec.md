# Spec Delta

## ADDED Requirements

### Requirement: Guest checks for OS and kernel identity

After a successful boot, verification SHALL confirm that the guest presents
as `obake-os` in `/etc/os-release`, that the login banner is
`Welcome to obake-os`, and that `uname -r` includes the expected
`-g<shortsha>[-dirty]` provenance suffix (and the board LOCALVERSION suffix
when the board under test defines one).

#### Scenario: os-release and issue in guest

- **WHEN** the installed host has booted in QEMU and guest functional checks
  run
- **THEN** the checks confirm `NAME`/`ID` are `obake-os`, version fields match
  `/etc/obake/image-version`, and `/etc/issue` contains `Welcome to obake-os`

#### Scenario: Kernel release provenance in guest

- **WHEN** the installed host has booted in QEMU and guest functional checks
  run for a board that defines a LOCALVERSION suffix
- **THEN** the checks confirm `uname -r` contains that board suffix and a
  `-g` git shortsha segment
