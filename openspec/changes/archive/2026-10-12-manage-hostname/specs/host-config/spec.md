# Spec Delta

## ADDED Requirements

### Requirement: obake renames the host without elevated privileges

The `obake` login user SHALL be able to set the host's static host name over
SSH without elevated privileges. The change SHALL take effect without a reboot
and SHALL persist across boots, slot switches, and operating system updates.
The image SHALL NOT rely on a hostname entry in `/etc/hosts`: the local name
SHALL resolve through the system resolver, so changing the host name does not
require editing `/etc/hosts`.

#### Scenario: obake sets the host name

- **WHEN** the `obake` user sets a new valid host name over SSH without sudo
- **THEN** the change succeeds and the host reports the new name

#### Scenario: Rename persists across a slot switch

- **WHEN** the `obake` user has renamed the host and the host then activates
  the other slot
- **THEN** the host still reports the new name

#### Scenario: No hostname entry in /etc/hosts

- **WHEN** an operator reads `/etc/hosts`
- **THEN** it does not map the host name, and the local host name still
  resolves

