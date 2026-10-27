# Spec Delta

## ADDED Requirements

### Requirement: Apply may remove a caller-supplied bundle

When `obake-update apply` is given an explicit local bundle path and the
`--cleanup` flag, the update client SHALL delete that path after a
successful install and SHALL NOT delete it when the install fails. When
`--cleanup` is set without an explicit caller-supplied path, the client
SHALL NOT delete a USB- or HTTPS-sourced bundle.

#### Scenario: Successful apply cleans up the path

- **WHEN** the `obake` user runs
  `obake-update apply --cleanup /home/.obake-update/bundle.raucb` and
  the install succeeds
- **THEN** that path no longer exists on the host

#### Scenario: Failed apply leaves the path

- **WHEN** the `obake` user runs
  `obake-update apply --cleanup /home/.obake-update/bundle.raucb` and
  the install fails
- **THEN** that path still exists and no slot has been activated for the
  failed install beyond RAUC's rejection behavior

#### Scenario: Cleanup without a path does not delete USB media

- **WHEN** the `obake` user runs `obake-update apply --cleanup` with no
  path and the install uses a USB-discovered bundle
- **THEN** the USB bundle file remains on the removable media

### Requirement: Apply may reboot after a successful install

When `obake-update apply` is given the `--reboot` flag and the install
succeeds, the update client SHALL request a system reboot. When the
install fails, the client SHALL NOT reboot. When both `--cleanup` and
`--reboot` are set with an explicit path, the client SHALL delete the
path before requesting the reboot.

#### Scenario: Successful apply reboots

- **WHEN** the `obake` user runs
  `obake-update apply --reboot /path/to/bundle.raucb` and the install
  succeeds
- **THEN** the host begins a system reboot

#### Scenario: Failed apply does not reboot

- **WHEN** the `obake` user runs
  `obake-update apply --reboot /path/to/bundle.raucb` and the install
  fails
- **THEN** the host does not reboot

#### Scenario: Cleanup runs before reboot

- **WHEN** the `obake` user runs
  `obake-update apply --cleanup --reboot /home/.obake-update/bundle.raucb`
  and the install succeeds
- **THEN** that path is removed and then the host begins a system reboot
