# Proposal

## Why

Verification and developer-VM containers still write under `image/_verify/` and
`image/_dev/` as container root, so host trees land as foreign-owned (often
`nobody`) and cannot be removed without elevated privileges. The build path
already runs as the invoking user; verification should match that contract.

## What Changes

- Run `image/verification/verify-env/run.sh` as the invoking user, with
  `--group-add` for the host groups that own `/dev/kvm` and
  `/var/run/docker.sock` when those devices exist.
- Document the host prerequisites (membership / access to KVM and the Docker
  socket) for containerized verification and for raw `docker run` examples
  that bypass `run.sh`.
- Widen the host-owned outputs requirement so every host-visible artifact from
  build, verification, and `_dev` workflows is owned by the invoking user and
  removable without elevated privileges.
- Provide a scoped ownership fallback (same idea as the build's
  `ensure_user_owned`) for leftover foreign-owned trees under `_verify` /
  `_dev` when a prior run left them behind.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `image-verification`: Extend host-owned outputs beyond the build container to
  verification and `_dev` host-visible artifacts; document verify-env identity
  and device-group requirements.

## Impact

- `image/verification/verify-env/run.sh` and its README / `dev-qemu.md` examples
- Possibly a small shared ownership helper used by verify-env (and optionally
  harness cleanup)
- Spec text under `openspec/specs/image-verification/`
- No change to guest image behavior, OTA, or persistent host configuration
