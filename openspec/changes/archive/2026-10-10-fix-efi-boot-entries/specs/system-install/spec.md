# Spec Delta

## MODIFIED Requirements

### Requirement: A/B boot installation

The installer SHALL install a unified kernel image for each slot and create
exactly one active firmware boot entry for each slot, labeled `A` and `B`,
pointing at that slot's UKI on the ESP it just wrote. Before creating those
entries it SHALL remove every existing firmware boot entry whose label is `A`
or `B`, including entries for a previous ESP and entries that are not active.
Entries with any other label SHALL be left in place. The resulting boot order
SHALL try the install's active slot before the other slot.

#### Scenario: First boot uses active slot

- **WHEN** installation completes and the host powers on
- **THEN** the firmware loads the active slot's unified kernel image and the
  corresponding root slot

#### Scenario: Reinstall does not keep stale slot entries

- **WHEN** the installer runs on a machine that already has one or more EFI
  entries labeled `A` or `B`, including entries that point at a different disk
  or are inactive
- **THEN** those entries are removed, exactly one active `A` and one active `B`
  remain, and both point at the UKIs on the ESP written by this install

#### Scenario: Other firmware entries survive install

- **WHEN** the firmware has boot entries whose labels are not `A` or `B`
- **THEN** the installer does not delete those entries
