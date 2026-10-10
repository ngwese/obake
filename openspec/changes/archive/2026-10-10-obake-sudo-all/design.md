# Design

## Context

See proposal.md for motivation. Today
`image/os/buildroot/board/obake/rootfs-overlay/etc/sudoers.d/obake-update`
grants only:

```
obake ALL=(root) NOPASSWD: /usr/bin/obake-update
```

`post-fakeroot.sh` already enforces root ownership and mode `0440` on
`etc/sudoers.d/*`. Guest check `check_update_sudo` in
`image/verification/guest/obake-verify.sh` asserts `obake-update` sudo works
and shell sudo is denied. Root remains locked via shadow; that stays.

## Goals / Non-Goals

**Goals:**
- Passwordless sudo for `obake` for all commands, including `sudo -s` / `-i`
- Keep root account locked (no empty-password `su`)
- Flip guest verification and docs to match the broader grant
- Preserve `obake-update` re-exec-via-sudo behavior under the new grant

**Non-Goals:**
- Unlocking or setting a root password
- Changing SSH auth policy for `obake` (keys / optional password)
- Remounting the root filesystem read-write by default
- Narrowing or removing polkit rules for systemd/power (they can remain)

## Decisions

### 1. Replace the narrow sudoers line with `NOPASSWD: ALL`

Change the drop-in to:

```
obake ALL=(root) NOPASSWD: ALL
```

Keep the file under `etc/sudoers.d/` (rename to `obake` optional but not
required; updating in place is fine). Comments should state that full sudo is
intentional for operator debugging, and that root stays locked.

Alternative considered: keep the narrow grant and add only `/bin/bash`,
`/bin/sh`, and a shell whitelist. Rejected — the request is all commands for
debugging, and a whitelist would keep growing.

Alternative considered: require a password for non-update sudo. Rejected —
hosts often use key-only SSH with no usable `obake` password, and the existing
update path is already NOPASSWD.

### 2. Keep root locked; sudo is the escalation path

Do not change `post-fakeroot.sh` root-lock logic. Operators escalate with
`sudo`, not `su`. Guest checks continue to assert empty-password `su` fails.

### 3. Invert the guest sudo assertion

In `check_update_sudo` (or rename to `check_sudo`):

- Keep: `sudo -n /usr/bin/obake-update status` must succeed
- Change: `sudo -n /bin/sh -c true` must succeed (was: must fail)

Optionally also probe `sudo -n -s -- true` / a trivial `id -u` under sudo
equals 0. Keep the check runnable non-interactively (`-n`).

### 4. Docs follow the grant

Update `image/docs/host-config.md`, `os-update.md`, and `user-guide.md` where
they say the sudo grant is limited to `obake-update` only. State that root
remains locked and that unprivileged writes still follow the directory
contract.

## Risks / Trade-offs

- [Anyone who can authenticate as `obake` gets full root via sudo] → Accept
  for debugging; mitigate with SSH key hygiene and optional `OBKA_PASSWORD`
  only when needed. Document the wider blast radius.
- [Operators may remount root RW and break the update contract] → Docs warn
  that slot images remain the source of truth; prefer `/persist` and
  `/home` for lasting changes.
- [Existing hosts keep the narrow grant until OTA] → Expected; no persist
  migration. New images / next update carry the drop-in.

## Migration Plan

1. Ship the new sudoers drop-in in the OS image overlay.
2. Update guest verification so CI fails if shell sudo is still denied.
3. Document the behavior change in host-config / os-update / user-guide.
4. Deploy via normal A/B update; no rollback special-case beyond reverting
   the image.
