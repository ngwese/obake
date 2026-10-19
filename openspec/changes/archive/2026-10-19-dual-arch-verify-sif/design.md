# Design

## Context

See proposal.md for motivation. Current state:

- `image/verification/fixtures/verify.def` bootstraps a tiny Alpine image
  (`%runscript` → `/bin/true`).
- A single `verify.sif` is committed (git-lfs) and is amd64.
- `install-verify.sh` `embed_verify_sif` always copies that file to
  `/opt/obake/verify.sif`.
- `post-build.sh` already detects `BR2_aarch64` / `BR2_x86_64` into `arch`
  (`aarch64` or `x86_64`) before calling `install-verify.sh`.
- Guest checks in `obake-verify.sh` only care about `/opt/obake/verify.sif`.

## Goals / Non-Goals

**Goals:**

- Two committed fixtures, selected at VERIFY install time by target arch.
- Unchanged guest path and check scripts.
- Clear regenerate instructions for both arches.

**Non-Goals:**

- Building the SIF inside the Buildroot image build.
- Multi-arch shapes or production container images.
- Changing Singularity setuid / fakeroot / SquashFS assertions beyond arch
  matching.

## Decisions

### Fixture names: `verify-x86_64.sif` and `verify-arm64.sif`

Use the names from the request. Map Buildroot / post-build `arch`:

| Target `arch` | Fixture                |
|---------------|------------------------|
| `x86_64`      | `verify-x86_64.sif`    |
| `aarch64`     | `verify-arm64.sif`     |

`arm64` in the filename matches Singularity's usual arch label; Buildroot's
`aarch64` stays the internal board/arch token.

Alternative considered: `verify-aarch64.sif` to match `OBKA_ARCH`. Rejected
to follow the requested naming.

### Select at embed time; keep guest path

`embed_verify_sif` reads target arch the same way `post-build.sh` does
(`BR2_CONFIG`: `BR2_aarch64=y` / `BR2_x86_64=y`), or accepts an optional
`VERIFY_SIF_ARCH` / `OBKA_ARCH` override if already set. It copies the
matching fixture to `/opt/obake/verify.sif`. Missing fixture or unknown arch
fails the VERIFY install loudly.

Guest scripts stay on `/opt/obake/verify.sif` — no changes to
`obake-verify.sh`.

### Remove `verify.sif`

Delete the old single-arch file so a stale LFS pointer cannot be embedded by
mistake. README documents rebuilding both:

```sh
# native or with binfmt/qemu-user as needed
singularity build --fakeroot --force verify-x86_64.sif verify.def
# for arm64 on an x86_64 builder, prefer a native arm64 host or
# docker/singularity platform support, e.g.:
SINGULARITY_DOCKER_PLATFORM=linux/arm64 \
  singularity build --fakeroot --force verify-arm64.sif verify.def
```

Exact flags may vary by Singularity CE version; README records what works in
this project's verify-env / common hosts. Both SIFs remain git-lfs via the
existing `verification/fixtures/*.sif` rule.

### Same `verify.def` for both

One definition file; architecture comes from the build host or Docker
platform, not from the def. No second def file.

## Risks / Trade-offs

- [Cross-building arm64 SIF on x86_64] → May need qemu-user / Docker
  platform emulation or a native arm64 builder. Document both paths; commit
  the binaries so CI consumers do not rebuild.
- [Stale cache still embedding old verify.sif] → Removing the file and
  failing closed on missing arch-specific names forces a correct embed.
- [Filename `arm64` vs internal `aarch64`] → Document the mapping next to
  `embed_verify_sif` to avoid future confusion.

## Migration Plan

Replace fixtures in-tree; next VERIFY rebuild picks the right SIF. No guest
data migration. Rollback is restoring the previous change / `verify.sif`.

## Open Questions

None.
