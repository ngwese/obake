# Proposal

## Why

The obake host OS lifecycle is documented across several subsystem files
(`image/os/README.md`, `image/installer/README.md`,
`image/verification/README.md`, and `image/docs/{host-config,os-update,
dev-qemu}.md`), but there is no single user-facing entry point that takes an
operator from a fresh board to a running, updated host. The pieces exist; the
through-line does not.

## What Changes

- Add `image/docs/user-guide.md`: one end-to-end guide to the obake-image OS
  setup covering the build system structure, building an image, what QEMU
  verifies and what only hardware can, creating and booting a USB installer
  for a bare-metal board, SSH key provisioning, the read-only root vs
  writable contract, and updating a host from a USB drive.
- Link the guide from `image/README.md` and cross-link the existing subsystem
  docs, which stay the detailed references; the guide synthesizes rather than
  duplicates them.
- Scope the bare-metal install section to the x86_64 `qdl` board. The
  `rk35xx`/arm64 board is excluded from that section because its installer
  boot path is unverified.
- Document the installer's interactive target-disk selection and the
  local-console path, which depend on the in-flight
  `add-installer-local-console` change.
- Documentation only: no build, installer, runtime, or script behavior
  changes.

## Capabilities

### New Capabilities

None. This is a documentation-only change with no spec-level behavior change,
so `.openspec.yaml` sets `skip_specs: true`.

### Modified Capabilities

None.

## Impact

- `image/docs/user-guide.md` (new) and an `image/README.md` link, committed in
  the `obake-image` repository (the `image/` submodule).
- Relates to `image/docs/host-config.md`, `image/docs/os-update.md`,
  `image/docs/dev-qemu.md`, `image/os/README.md`,
  `image/installer/README.md`, and `image/verification/README.md`; those stay
  authoritative and the guide links to them.
- Depends on `add-installer-local-console` for the `INSTALLER_CONSOLE=local`
  and interactive disk-selection behavior it documents.
- No effect on persistent configuration, OTA upgrades, or cross-host
  portability; the guide only describes existing behavior.
