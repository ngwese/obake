# Design

## Context

See proposal.md for motivation. On-device updates already work through
`image/.../usr/bin/obake-update` (`status` / `apply` / `rollback`) and
RAUC A/B slots. Bundle artifacts are named `obake-<semver>-{a,b}.raucb`
with no board/arch in the filename. The `obake` CLI already shells out
to `ssh` for `auth login`; host config stores HTTPS credentials only.
`obake-session` GraphQL has no OS/update surface, and this change keeps
update out-of-band from that daemon.

## Goals / Non-Goals

**Goals:**

- One operator command that probes slot/version over SSH, picks the
  right local `.raucb`, pushes it, applies with cleanup+reboot.
- Small, explicit extensions to `obake-update apply` (`--cleanup`,
  `--reboot`) usable without the CLI.
- Fail closed on wrong slot / older semver unless `--force` on a file.

**Non-Goals:**

- `device describe`, GraphQL device API, remembering SSH identity in
  client config, board/arch filename checks, changing RAUC layout or
  health-gate behavior, auto-discovery of build output trees beyond the
  provided `PATH`.

## Decisions

### 1. SSH-only transport with required `--target`

`obake device update --target user@host PATH [--force]` always uses the
system `ssh`/`scp` (or `scp`-equivalent via `ssh`). No `--host` / bearer
path for this noun.

Alternatives considered: GraphQL mutations (rejected — session may be
down); reuse config host name as SSH host with implied `obake@`
(deferred — implicit and wrong when login user differs).

### 2. Probe via `obake-update status` text

Parse `running version:` and `active slot:` from `obake-update status`
over SSH (same strings the on-device tool already prints). Map
`rootfs.0 (A)` / `Booted from: rootfs.0` style output to inactive
slot letter `b`, and B → `a`. Prefer robust parsing of the `A`/`B`
marker already embedded in status output.

Alternatives: ad-hoc `cat /etc/obake/image-version` + `rauc status`
(more brittle duplication); machine-readable JSON status (nice later,
out of scope).

### 3. Bundle filename contract

Match `^obake-([0-9]+\.[0-9]+\.[0-9]+)-([ab])\.raucb$` (strict semver
MAJOR.MINOR.PATCH, no prerelease). Compare versions numerically by
component. Directory mode: filter by inactive slot, keep version ≥
device, take max. File mode: enforce slot + version ≥ unless `--force`.
`--force` on a directory is an error (or ignored with a warning — prefer
hard error so operators do not think checks were skipped while
selection still filtered).

### 4. Stage path on device

SCP to `/home/.obake-update/<basename>` when `/home` is the usual user
partition (same directory `obake-update` already uses for HTTPS
downloads). Fall back is unnecessary if `obake` user's home exists on
production images; if remote mkdir fails, surface the SSH error.

### 5. `obake-update apply` flag parsing

Extend usage to:

```
obake-update apply [--cleanup] [--reboot] [<bundle-path>]
```

Flags may appear before or after `apply`'s operands in whatever form is
natural for the shell script (parse known flags from `$@` after the
subcommand). On success of `rauc install` for an explicit path:
1. if `--cleanup`, `rm -f` that path (only that path);
2. if `--reboot`, `systemctl reboot`.
On failure, skip both. CLI always invokes
`obake-update apply --cleanup --reboot <staged>`.

### 6. CLI module layout

New `services/obake/src/device.rs` (or `update.rs`) owning: status
probe, filename parse/select, scp, remote apply. Wire
`Commands::Device { action: DeviceCmd::Update { target, path, force } }`
in `main.rs`, short-circuit before GraphQL resolution like `auth`.
Unit-test selection/validation without SSH; integration can stub `ssh`
via PATH in tests where practical.

### 7. Docs

Update `services/obake/README.md`, `image/docs/os-update.md`, and the
user-guide update section so the preferred laptop path is
`obake device update --target …`.

## Risks / Trade-offs

- **[Risk] Status text format drift** → Mitigation: parse only the two
  labeled lines `obake-update status` owns; add a CLI unit fixture with
  sample output; keep status format stable when editing the script.
- **[Risk] Reboot drops SSH mid-command** → Mitigation: treat connection
  loss after a successful apply as expected when `--reboot` is used;
  CLI should report success if remote apply exited 0 before reboot, or
  document that a dropped connection after invoke is normal.
- **[Risk] Wrong-board bundle still pushed** → Mitigation: RAUC
  signature/compatible check rejects it; out of scope to encode board
  in filenames.
- **[Risk] Semver-only filenames reject build metadata versions** →
  Mitigation: image VERSION is already `MAJOR.MINOR.PATCH`; record
  assumption in tasks; widen later if needed.
- **[Trade-off] No saved SSH target** → Operators type `--target` each
  time; keeps config schema and auth model unchanged.

## Migration Plan

1. Ship updated `obake-update` in the next OS image (flags no-op for
   callers that omit them).
2. Ship `obake` CLI with `device update` that requires the new flags on
   the device — operators updating *to* the first image that includes
   `--cleanup`/`--reboot` still use manual apply once; afterward the
   CLI path works. Document that the device must already run an image
   whose `obake-update` understands the flags (or the CLI falls back is
   explicitly not provided — fail with the remote usage error).
3. No rollback beyond reverting the CLI package / next image.

## Open Questions

None blocking. Optional later: JSON status from `obake-update`, SSH
identity in client config, `device describe`.
