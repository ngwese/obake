# Design

## Context

See proposal.md for motivation. Today `board/obake/users.txt` creates `obake`
with password field `*`, so password login is disabled and access is by SSH
key (`OBKA_SSH_AUTHORIZED_KEY` → `/etc/ssh/authorized_keys.d/obake`, also
seeded onto persist at install). Build knobs are forwarded through
`image/build/build.sh` and `image/build/build-env/run.sh` into the Buildroot
container. Rootfs ownership that must be root is applied in
`post-fakeroot.sh` inside Buildroot's fakeroot session.

## Goals / Non-Goals

**Goals:**

- Optional `OBKA_PASSWORD` that sets the `obake` account password in the built
  rootfs when non-empty and enables SSH password authentication as `obake`.
- Preserve default key-only behavior when the knob is unset or empty.
- Forward the knob through the same build/verify env path as other `OBKA_*`
  settings, without logging the cleartext value.

**Non-Goals:**

- Seeding or rotating the password on the persist partition.
- Generating or storing an SSH keypair at build time.
- Changing polkit/sudo passwordless maintenance for `obake`.
- Forcing `PasswordAuthentication no` in sshd when no password is set
  (shadow already rejects password auth).
- Runtime password change tooling or first-boot password prompts.

## Decisions

1. **Knob name `OBKA_PASSWORD`** — cleartext in the environment, matching the
   existing `OBKA_*` build-knob style. Alternative considered: a path
   (`OBKA_PASSWORD_FILE`) like the SSH key; deferred unless operators need to
   avoid env visibility in `docker -e` listings. Document that the value is
   sensitive and should not be committed.

2. **Apply in `post-fakeroot.sh`, leave `users.txt` as `*`** — keep the
   tracked users table password-disabled by default; when `OBKA_PASSWORD` is
   non-empty, set the hash on the assembled rootfs inside fakeroot (e.g.
   `chpasswd --root` or host `mkpasswd` + shadow edit). Alternatives: rewrite
   `users.txt` before `make` (mutates inputs / complicates incremental
   builds), or a generated users fragment (more Buildroot wiring for little
   gain).

3. **Image-only, not persist** — password is a property of the slot image,
   same as the rest of `/etc/shadow`. SSH keys stay on persist because they
   are machine-operator identity across updates; a build-time password is
   chosen per image and correctly replaced by OTA. Alternative: copy a hash
   onto persist (rejected — duplicates credentials and fights A/B updates).

4. **Enable SSH password auth when a password is set** — when
   `OBKA_PASSWORD` is non-empty, ensure sshd allows password authentication
   (e.g. a drop-in under `sshd_config.d` with `PasswordAuthentication yes`,
   written from post-build/post-fakeroot alongside the shadow update). When
   the knob is unset, leave sshd as today so key-only access is unchanged.
   Alternative rejected: rely on OpenSSH package defaults alone (not an
   explicit guarantee that password SSH works).

5. **Verification via QEMU SSH password login** — headless check: build with
   a test `OBKA_PASSWORD`, boot under QEMU, and authenticate as `obake` over
   SSH using that password (prefer password auth, not the authorized key).
   Also assert a default build keeps password login disabled (shadow
   `*`/`!` and/or failed password SSH). Shadow inspection alone is not
   enough to prove sshd accepts password auth.

## Risks / Trade-offs

- **[Risk] Cleartext in process environment / docker `-e`** → Mitigation:
  document sensitivity; never print the value in build logs; consider a file
  knob later if needed.
- **[Risk] Password hash recoverable from the image artifact** → Mitigation:
  document that anyone with the image can offline-attack the hash; treat
  build-time passwords as shared device secrets, not high-value credentials.
- **[Risk] OTA replaces the password with the new image's setting** →
  Mitigation: document as intended; operators who need a stable password must
  bake the same value into update images.

## Migration Plan

No migration for existing hosts: default builds stay passwordless. Operators
who want a password rebuild (and reinstall or OTA) with `OBKA_PASSWORD` set.
Rollback is building without the knob (or with empty value).
