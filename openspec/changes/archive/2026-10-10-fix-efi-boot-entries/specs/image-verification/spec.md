# Spec Delta

## ADDED Requirements

### Requirement: EFI slot entries reconcile without duplicates

Verification SHALL run a headless check of the EFI slot-entry reconcile rules.
The check SHALL fail if a stale, inactive, wrong-disk, or duplicate `A` or `B`
entry would remain, or if it would be ordered ahead of the current ESP's entry
for that slot. The check SHALL also fail if an entry whose label is not `A` or
`B` would be deleted.

#### Scenario: Stale slot entry is not kept

- **WHEN** the reconcile check is given a boot list that includes an older `A`
  entry on a different disk ahead of the current ESP's `A` and `B` entries
- **THEN** the check passes only if the result has one `A` and one `B` for the
  current ESP and does not keep the older entry

#### Scenario: Unrelated entries are kept

- **WHEN** the reconcile check is given a boot list that also contains an entry
  whose label is not `A` or `B`
- **THEN** the check passes only if that entry is still present
