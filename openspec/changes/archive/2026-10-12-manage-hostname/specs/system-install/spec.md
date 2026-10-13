# Spec Delta

## ADDED Requirements

### Requirement: Install-time hostname selection

The installer SHALL determine the host name from a seed that defaults to
`obake`. When the install is interactive it SHALL prompt on the selected
console for the host name, offering the seed as the default and accepting an
empty answer as the default. When the install is non-interactive it SHALL use
the seed without prompting. The installer SHALL accept only valid RFC1123 host
names: an invalid interactive entry SHALL re-prompt, and an invalid
non-interactive seed SHALL fail the install. The chosen name SHALL become the
installed host's static host name and SHALL survive updates and slot switches.

#### Scenario: Interactive prompt with default

- **WHEN** the installer runs interactively and the operator answers the
  hostname prompt with a valid name
- **THEN** the installed host boots with that name

#### Scenario: Interactive empty answer keeps the seed

- **WHEN** the installer runs interactively and the operator answers the
  hostname prompt with nothing
- **THEN** the installed host keeps the seed host name

#### Scenario: Non-interactive uses the seed

- **WHEN** the installer runs non-interactively
- **THEN** it does not prompt and the installed host uses the seed host name

#### Scenario: Invalid name is rejected

- **WHEN** the operator enters a host name that is not valid, or the seed is
  not valid and the install is non-interactive
- **THEN** the installer re-prompts (interactive) or fails the install
  (non-interactive), and no invalid name is applied

## MODIFIED Requirements

### Requirement: Machine-specific seeding

The installer SHALL seed machine-specific state including a machine
identifier, SSH host keys, hostname, and network configuration into the layer
of the persist partition that the installed image's `/etc` overlay uses, so the
seed takes effect on the installed host, and SHALL create the user partition.

#### Scenario: Persist seeded at install

- **WHEN** installation completes
- **THEN** the persist partition contains a unique machine identifier, SSH host
  keys, and initial hostname and network configuration in the overlay layer
  that the installed image's `/etc` overlay reads, and the user partition
  exists and is mountable

#### Scenario: Seeded identity takes effect

- **WHEN** the installed host boots after an install that seeded an identity
- **THEN** `/etc` reflects the seeded host name, network configuration, host
  keys, and machine identifier
