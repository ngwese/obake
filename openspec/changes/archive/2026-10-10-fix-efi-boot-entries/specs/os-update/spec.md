# Spec Delta

## MODIFIED Requirements

### Requirement: A/B slot application with kernel

The system SHALL write an update to the inactive root slot together with the
matching per-slot unified kernel image, and SHALL activate the updated slot on
the next boot without modifying the currently active slot. The firmware entries
used for that activation SHALL be the single active `A` and `B` entries for the
mounted ESP's slot UKIs. On a later boot, when no one-shot next boot is set,
the booted slot's entry SHALL be ordered ahead of the other slot so the slot
that ran remains the one selected for the following boot.

#### Scenario: Inactive slot written

- **WHEN** an update is applied
- **THEN** only the inactive slot and its unified kernel image are written

#### Scenario: Kernel updates with userspace

- **WHEN** a bundle contains a new kernel
- **THEN** the new kernel is installed as part of the same slot activation so
  kernel and userspace change together

#### Scenario: Activation ignores stale slot entries

- **WHEN** an update is applied while older EFI entries labeled `A` or `B`
  still exist for another disk, an inactive option, or a non-file path
- **THEN** the next boot is directed at the current ESP's entry for the updated
  slot, not at one of those stale entries

#### Scenario: Booted slot stays selected

- **WHEN** the host has booted a slot and no one-shot next-boot entry is set
- **THEN** the firmware boot order tries that slot before the other slot on the
  following boot
