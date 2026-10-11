# Design

## Context

See proposal.md for motivation. Shapes are multi-stage Debian bookworm
Singularity images that drop compilers in the final stage
(`shapes/<name>/<name>.def`). The host OS image is built by
`image/build/build.sh` inside the Docker image `obake-buildroot-env`. Neither
is a compiler environment. Singularity definitions in this repo bootstrap with
`Bootstrap: docker` and `From: debian:<suite>` (shapes) or, for siren, with
`Bootstrap: library`.

## Goals / Non-Goals

**Goals:**

- One Singularity definition that builds an immutable compiler SIF.
- C, C++, Clang, CMake, and a rustup stable toolchain usable from `PATH`.
- Cargo's download cache in the invoking user's home, so a read-only SIF can
  still fetch crates.

**Non-Goals:**

- sudo, writable overlays, sandboxes, or installing packages after the SIF
  exists.
- A pinned Rust version, a cross toolchain, or a sysroot for bookworm shapes
  or the Buildroot host image.
- Shape labels, a build wrapper script, or further builder definitions.
- Replacing `image/build/`.

## Decisions

### Place the definition beside shapes, in its own tree

`builders/builder-base/builder-base.def` mirrors `shapes/<name>/<name>.def`,
so a later builder can keep helper files next to its definition.

Alternative: a single `builders/builder-base.def`. Rejected because the
nested layout is already how shapes grow past one file.

### One Docker stage from trixie-slim

```
Bootstrap: docker
From: debian:trixie-slim
```

No build/final split. The image that runs is the image that contains the
compilers. `trixie-slim` is the requested base; the installed toolchain is
what makes the result larger than slim.

Alternative: bookworm, matching shapes. Rejected because the requested base
is trixie. Binaries linked in this image are not expected to run inside
bookworm shape finals.

### Install a linking C/C++ toolchain plus the named tools

`%post` installs, noninteractively:

- `build-essential` (`gcc`, `g++`, `make`, `libc6-dev`)
- `binutils`, `clang`, `cmake`, `curl`, `wget`, `ca-certificates`, `python3`,
  `git`

`clang` provides both `clang` and `clang++`. `libc6-dev` is required for
`gcc`/`g++` to link. `ca-certificates` is required for the rustup HTTPS
download. `sudo` is not installed. Apt lists may be removed after install;
the image does not apt-install later.

Alternative: name `gcc`, `g++`, `make`, and `libc6-dev` separately. Equivalent
to `build-essential` for this set; the metapackage is the Debian way to get a
working compiler.

### Install rustup as root, keep Cargo's home on the host

During `%post`, as root:

- `RUSTUP_HOME=/usr/local/rustup`
- `CARGO_HOME=/usr/local/cargo`
- Official rustup installer, noninteractive, `--no-modify-path`,
  `--default-toolchain stable`, default profile
- `chmod -R a+rX` on both trees so the invoking user can execute them

`%environment` then sets:

- `RUSTUP_HOME=/usr/local/rustup`
- `CARGO_HOME=$HOME/.cargo`
- `PATH` prefixed with `/usr/local/cargo/bin`

Proxies stay in the image. Registry and git checkouts go to the home
directory Singularity bind-mounts. The stable channel is whatever rustup
resolves when the image is built; it is not pinned.

Alternative: Debian `rustc`. Rejected because the request is rustup and the
current stable toolchain. Alternative: world-writable `/usr/local/cargo`.
Rejected because that cache would need a writable image root.

### Ship a SIF and nothing else

`singularity build` of the definition produces a read-only SIF. No sudoers
entry, no `--writable` run instructions, no overlay.

## Risks / Trade-offs

- [Risk] A later rebuild resolves a newer stable Rust → Mitigation: accepted.
  Pin a toolchain version in a follow-up if rebuilds must match.
- [Risk] An x86_64 SIF and an aarch64 SIF are different images → Mitigation:
  the host triple is the build machine's. Do not reuse one SIF across
  architectures.
- [Risk] Objects linked here use trixie glibc and will not run in bookworm
  shapes or the Buildroot host → Mitigation: this image is the place the
  build runs, not a sysroot for those targets.
- [Risk] The rustup installer is fetched from the network at build time →
  Mitigation: use the official installer URL over HTTPS. The build requires
  network access, as Debian package installation does.

## Migration Plan

Add `builders/builder-base/builder-base.def`. No existing images, host
configuration, or update artifacts change. Rollback is removing that
directory.
