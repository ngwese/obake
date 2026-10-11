# Spec Delta

## MODIFIED Requirements

### Requirement: Guest checks for OS and kernel identity

After a successful boot, verification SHALL confirm that the guest presents
as `obake-os` in `/etc/os-release`, that the login banner is
`Welcome to obake-os`, that `VERSION` equals `/etc/obake/image-version`, that
`VERSION_ID` is that version plus `-g<shortsha>`, `-g<shortsha>-dirty`, or
`-gnogit`, that `PRETTY_NAME` is `obake-os` followed by `VERSION_ID`, and that
`uname -r` ends with `-<board>-<rev>` for the board under test, where `<rev>`
is a positive integer. `uname -r` SHALL NOT contain an image git commit.

#### Scenario: os-release and issue in guest

- **WHEN** the installed host has booted in QEMU and guest functional checks
  run
- **THEN** the checks confirm `NAME`/`ID` are `obake-os`, `VERSION` equals
  `/etc/obake/image-version`, `VERSION_ID` is that version plus a `-g`
  git suffix (`-g<shortsha>`, `-g<shortsha>-dirty`, or `-gnogit`),
  `PRETTY_NAME` is `obake-os` followed by `VERSION_ID`, and `/etc/issue`
  contains `Welcome to obake-os`

#### Scenario: Kernel release provenance in guest

- **WHEN** the installed host has booted in QEMU and guest functional checks
  run
- **THEN** the checks confirm `uname -r` ends with the board id under test
  and a positive integer rev, and does not contain an image git short commit
