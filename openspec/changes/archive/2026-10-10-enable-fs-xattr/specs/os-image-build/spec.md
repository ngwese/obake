# Spec Delta

## ADDED Requirements

### Requirement: Filesystem extended attribute support

Every board kernel SHALL enable extended-attribute support for each onboard
filesystem that exposes a kernel option for it among the filesystems the image
already requires: SquashFS xattrs, ext4 POSIX ACLs and security xattrs, and
tmpfs xattrs (with tmpfs POSIX ACLs). Filesystems without a Linux xattr Kconfig
knob (for example FAT/exFAT) are unchanged. Overlay and FUSE SHALL continue to
rely on their stacked or userspace xattr paths without needing a separate
host-only xattr toggle.

#### Scenario: SquashFS xattrs enabled

- **WHEN** a board kernel is built with the shared host fragment
- **THEN** SquashFS extended-attribute support is enabled

#### Scenario: ext4 ACL and security xattrs enabled

- **WHEN** a board kernel is built with the shared host fragment
- **THEN** ext4 POSIX ACL support and ext4 security-label (security xattr)
  support are enabled

#### Scenario: tmpfs xattrs enabled

- **WHEN** a board kernel is built with the shared host fragment
- **THEN** tmpfs extended-attribute support is enabled

#### Scenario: SIF with xattrs mounts without ignore errors

- **WHEN** a booted image mounts a SquashFS-backed SIF that contains extended
  attributes using the kernel SquashFS driver
- **THEN** the kernel does not log that SquashFS xattrs will be ignored or that
  the xattr id index table cannot be read
