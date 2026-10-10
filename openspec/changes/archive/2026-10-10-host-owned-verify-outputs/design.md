# Design

## Context

See proposal.md for motivation. The build container already runs with
`--user "$(id -u):$(id -g)"` (`image/build/build-env/run.sh`) and stages
`_out` with an `ensure_user_owned` fallback. `image/verification/verify-env/run.sh`
starts as container root against a bind-mounted repo, so `_verify` / `_dev`
trees become foreign-owned (uid 65534 / `nobody` under rootless Docker on the
current WSL host). Harness `rm -rf "$out"` and normal cleanup then fail.

## Goals / Non-Goals

**Goals:**

- Make verify-env identity match the build-env pattern.
- Keep KVM and Docker-socket access working for non-root container users.
- Leave a recovery path for trees already foreign-owned.
- Align docs with the real host prerequisites.

**Non-Goals:**

- Changing harness logic or guest checks.
- Fixing ownership for ad-hoc `docker run` invocations that bypass `run.sh`
  (docs will show the equivalent flags; behavior is best-effort).
- Migrating named Docker volumes used only by the build cache (already
  handled in build-env).

## Decisions

1. **Run verify-env as `--user uid:gid`.** Same contract as build-env.
   Alternative: keep root and `chown -R` at exit — rejected as primary
   approach because mid-run cleanup (`rm -rf` at harness start) still fails
   and ownership is only correct after a successful finish.

2. **`--group-add` for kvm and docker.sock gids when paths exist.** Resolve
   with `stat` on the host before `docker run`. Omit the flag when the path
   is absent (TCG-only or no nested Docker). Alternative: require the host
   user to already be in those groups — rejected because the container's
   supplementary groups are independent of the host user's membership, and
   today's root-in-container path did not depend on host group membership.

3. **HOME / passwd bootstrap as needed.** The verify image may lack the
   invoker's uid in `/etc/passwd`. Prefer a writable HOME under the mounted
   tree or `/tmp` if a tool requires `$HOME`, mirroring build-env's
   `HOME=/cache/home` pattern only if something in the harness path actually
   needs it. Spike during implement if ssh-keygen or similar complains.

4. **Scoped ownership fallback.** After a verify-env run (or via a small
   helper the wrapper can call), if `_verify` / `_dev` subtrees written by
   that run are not owned by the invoker, `chown` via a root one-shot
   container (same pattern as `ensure_user_owned` in `image/build/build.sh`).
   Also usable once to reclaim the existing foreign-owned tree.

5. **Docs.** Update `image/verification/README.md` and
   `image/docs/dev-qemu.md`: prefer `verify-env/run.sh`; show `--user` and
   `--group-add` on any raw `docker run` examples that remain.

## Risks / Trade-offs

- **[Risk] Non-root container cannot use `/dev/kvm` or the Docker socket** →
  Mitigate with `--group-add`; document that the socket and device must be
  present and group-readable by those gids on the host.
- **[Risk] Nested `docker` CLI needs the binary + socket** → Keep existing
  bind-mounts of `docker` and the socket; group-add covers permission.
- **[Risk] Leftover nobody-owned trees block the first re-run** → Ownership
  fallback / one-shot reclaim before or after the wrapper runs.
- **[Trade-off] Host user not in `kvm`/`docker` still works via group-add** →
  Intentional; matches prior root-in-container capability without requiring
  host usermod.

## Migration Plan

1. Land verify-env identity + group-add + docs.
2. Reclaim existing `_verify` (and `_dev` if present) once via the chown
   helper so developers are not stuck needing sudo.
3. No rollback beyond reverting the wrapper; worst case is return to
   foreign-owned outputs.
