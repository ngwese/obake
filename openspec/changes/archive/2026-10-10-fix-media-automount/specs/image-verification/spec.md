## ADDED Requirements

### Requirement: Writable media mount root checked after boot

After a successful boot, the verification SHALL confirm that `/media` is a
writable mount root suitable for removable-media automount (not merely an
empty directory on the read-only root), so a read-only `/media` fails
headless verification before release.

#### Scenario: Media root is writable

- **WHEN** guest functional checks run after boot
- **THEN** the suite fails unless `/media` is mounted and a privileged probe
  can create and remove a directory under `/media`
