# Design

## Context

See proposal.md for motivation. Behavior that shapes the approach:

- The `/etc` overlay used at runtime has its upper layer at
  `persist/etc-overlay/upper` (`image/os/buildroot/board/obake/initramfs/init`),
  but `install.sh` `seed_identity` writes hostname, network configuration,
  machine-id, and SSH host keys to `persist/etc`, which the runtime never
  reads. The image default and boot-time generators mask the dead seed.
- `OBKA_HOSTNAME` exists in `install.sh` but is not forwarded by `build.sh`,
  so it is always the default `obake`.
- systemd is built with `hostnamed` (default `y`); `hostnamectl` and
  `systemd-hostnamed` are present. `49-obake.rules` grants systemd unit
  management and power over SSH but not the hostname actions.
- `/etc/hosts` maps `127.0.1.1 obake`; host resolution is
  `resolve [!UNAVAIL=return] files myhostname ...` with `libnss_myhostname`
  and `libnss_resolve` installed, so the local name resolves without it.
- `make-disk.sh` assembles a bootable disk without mounting, seeding the user
  partition with `mke2fs -d` under `fakeroot`; the persist partition is created
  empty.

## Goals / Non-Goals

**Goals:**
- `obake` renames the host over SSH without elevated privileges; the change
  persists across boots, slot switches, and updates.
- Hostname is selectable at install: interactive prompt (defaulted) or a
  non-interactive seed baked into the medium; `MAKE_DISK` images bake the seed.
- The documented identity seeds actually take effect (correct over quick).
- No hostname entry in `/etc/hosts`.

**Non-Goals:**
- Renaming via the physical console without logging in.
- Changing the machine-id or host-key generation policy beyond making the seed
  effective.
- A pretty-hostname/icon-name/chassis model; only the static host name.

## Decisions

### 1. Post-install rename via a polkit grant

Add `org.freedesktop.hostname1.set-static-hostname` (and `set-hostname`) to
`49-obake.rules`, so `obake` runs `hostnamectl set-hostname <name>` over SSH
without sudo. Rationale: matches the existing unit-management grant and needs
no new binary or elevated step. `systemd-hostnamed`, running as root, writes
`/etc/hostname` into the persist overlay, so the change persists.

Alternatives: a setuid helper (new surface); documenting `sudo hostnamectl`
(elevated, and `obake` already has full sudo, so this adds no capability but
keeps the rename out of the self-service model).

### 2. Hostname seed flows through the existing build knob

Forward `OBKA_HOSTNAME` through `build.sh` and `build-env/run.sh` like
`OBKA_SSH_AUTHORIZED_KEY`, validate it as an RFC1123 name at build time, have
`make-installer.sh` place it in the installer service environment, and have
`make-disk.sh` bake it into the disk's persist overlay. `install.sh` resolves
the final name from the seed and an optional prompt.

Alternatives: a per-install config file on removable media (deferred; the seed
covers the requested non-interactive case).

### 3. Identity seeding targets the overlay the runtime uses

Re-point `seed_identity` from `persist/etc` to the persist `/etc` overlay upper
(`persist/etc-overlay/upper`). Build the identity in a staging root (so
`systemd-machine-id-setup --root=<stage>` and `ssh-keygen` run normally) and
copy `etc/*` into the upper with root ownership and correct modes (private
host keys `0600`, `/etc/shadow`-style files unchanged). This makes hostname,
network configuration, machine-id, and SSH host keys take effect while keeping
the per-filesystem overlay semantics (upper files shadow the same lower paths).

Alternatives: fix only the hostname (rejected: leaves the documented seed
dead); write into the slot image (rejected: replaced on OTA).

### 4. Interactive and non-interactive selection

`install.sh` treats the install as interactive using the same condition it
already uses for disk selection (no fixed `TARGET_DISK`). Interactive installs
prompt on the selected console, offering the seed (`OBKA_HOSTNAME` or `obake`)
as the default and accepting Enter for it; non-interactive installs use the
seed silently. A valid name is required: interactive re-prompts, non-interactive
fails. Validation uses the RFC1123 rule systemd applies
(`hostname_is_valid`): 1-63 characters per label, letters/digits/hyphen, not
starting or ending with a hyphen.

### 5. `MAKE_DISK` bakes only the hostname

`make-disk.sh` writes `persist/etc-overlay/upper/hostname` (and creates
`etc-overlay/work`) only when a hostname seed is set. It does not bake
machine-id or SSH host keys: a generic image flashed to many hosts must not
share a per-host identity, so those stay boot-time generated. The installer,
which runs per host, seeds the full identity.

### 6. Drop the hostname from `/etc/hosts`

Remove the `127.0.1.1 <host>` line from the image's `/etc/hosts`. Local name
resolution comes from `systemd-resolved` and `myhostname`, so the line is not
needed and would otherwise drift from a renamed host. `localhost` and the IPv6
loopback entries stay.

## Risks / Trade-offs

- **[polkit action names]** → If the action id is wrong the grant silently does
  nothing. Verify `hostnamectl set-hostname` as `obake` over SSH in the
  harness.
- **[Overlay upper ownership/modes]** → Files written into
  `persist/etc-overlay/upper` must be root-owned with correct modes; private
  host keys `0600`. Verify a booted host's `/etc/ssh` and `/etc/hostname`
  ownership.
- **[machine-id semantics]** → Seeding machine-id into the overlay upper must
  produce a valid single-line id; generate it with a staging root rather than
  hand-rolling.
- **[Interactive prompt regressions]** → Non-interactive installs (fixed
  `INSTALLER_TARGET_DISK`) must not prompt and must not fail on an unset seed.
- **[Existing hosts]** → Prior installs keep the old `/etc/hosts` until an OTA
  replaces the slot image, and keep using `sudo hostnamectl` until the new
  polkit rule lands. No forced migration; docs note the change.
- **[Validate rule duplication]** → The build and the installer both validate;
  keep one implementation so messages agree.

## Migration Plan

New images and installs use the new seeding and grant. Existing hosts rename
with `sudo hostnamectl` until an OTA delivers the new polkit rule and
`/etc/hosts`. Rollback is reverting the scripts; a rename already written to
`/etc/hostname` in the overlay remains valid.

## Open Questions

None blocking. Removable-media hostname provisioning remains a future option.
