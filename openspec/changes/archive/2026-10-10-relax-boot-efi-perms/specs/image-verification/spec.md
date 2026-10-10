# Spec Delta

## ADDED Requirements

### Requirement: ESP access checked after boot

After a successful boot, the verification SHALL confirm that the `obake` user
can access `/boot/efi` without sudo, so an owner-only ESP mount fails headless
verification before release.

#### Scenario: Guest can access ESP as obake

- **WHEN** guest functional checks run after boot
- **THEN** the suite fails unless `obake` can list `/boot/efi` without sudo
