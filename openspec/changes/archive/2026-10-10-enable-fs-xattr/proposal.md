# Proposal

## Why

Mounting common Singularity SIFs (for example `library://lolcow`) via kernel
SquashFS logs `SQUASHFS error: Xattrs in filesystem, these will be ignored`
and `unable to read xattr id index table` because the host kernel lacks
SquashFS xattr support. Containers and the writable contract also need
extended attributes on the other filesystems the appliance mounts (ext4
POSIX ACL / security labels, tmpfs xattrs already partly enabled). Enable
xattr support for every host filesystem that has a kernel option, so SIF
mounts stay quiet and xattrs work on root, overlay, and tmpfs paths.

## What Changes

- Enable filesystem xattr-related kernel options in the shared host fragment
  for every onboard filesystem that exposes them: SquashFS xattrs, ext4 POSIX
  ACLs and security xattrs, and keep/reinforce tmpfs xattrs (and POSIX ACL).
- Document that FAT/exFAT (and similar) have no Linux xattr Kconfig knobs;
  overlay and FUSE use underlying/userspace xattr support rather than a
  separate host toggle.
- Rebuild board kernels so the options land in shipped images; verify at least
  one board’s built kernel config (and/or guest) shows the options and that a
  SquashFS SIF with xattrs no longer produces the ignore/index-table errors.
- No login, persist, or OTA protocol changes; next signed slot picks up the
  new kernel.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `os-image-build`: require host kernels to enable extended-attribute support
  for SquashFS, ext4, and tmpfs (the filesystems that offer kernel xattr
  options among those the image enables).

## Impact

- `image/os/buildroot/board/obake/linux-obake.fragment` (primary).
- Board `linux.config` files only if a board base config conflicts or omits
  something the fragment cannot force.
- Kernel rebuild for each board; OTA delivers the new kernel with the next
  signed image. No shape rebuilds.
- Out of scope: SELinux/AppArmor policy, userspace `attr` package installs,
  changing FAT/exFAT features, or new filesystem types.
