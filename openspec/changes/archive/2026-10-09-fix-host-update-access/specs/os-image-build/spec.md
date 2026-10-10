# Spec Delta

## ADDED Requirements

### Requirement: Removable filesystem support including FAT family and exFAT

The shared base image SHALL include kernel and userspace support needed to
mount common removable filesystems used for update media, including MS-DOS
FAT (`msdos`), the generic FAT stack, FAT32/VFAT (`vfat`), and exFAT, so
attaching such media does not fail with an unknown filesystem type.

#### Scenario: FAT family kernel options enabled

- **WHEN** an image is built
- **THEN** the kernel configuration enables `CONFIG_FAT_FS`,
  `CONFIG_MSDOS_FS`, and `CONFIG_VFAT_FS` for the ESP and for
  MS-DOS/FAT/FAT32 removable media

#### Scenario: exFAT mount type is known

- **WHEN** an image is built and a removable exFAT volume is attached
- **THEN** the system recognizes filesystem type `exfat` and can mount the
  volume (via the removable-media auto-mount path)
