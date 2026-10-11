# Tasks

## 1. builder-base definition

- [x] 1.1 Add `builders/builder-base/builder-base.def` as a single-stage
      `Bootstrap: docker` / `From: debian:trixie-slim` definition. In `%post`,
      install `build-essential`, `binutils`, `clang`, `cmake`, `curl`,
      `wget`, `ca-certificates`, `python3`, and `git`, and do not install
      `sudo`. Install rustup noninteractively to `RUSTUP_HOME=/usr/local/rustup`
      and `CARGO_HOME=/usr/local/cargo` with the stable default toolchain,
      world-readable. In `%environment`, set `RUSTUP_HOME=/usr/local/rustup`,
      `CARGO_HOME=$HOME/.cargo`, and prefix `PATH` with `/usr/local/cargo/bin`.
      Leave `shapes/` and `image/` unchanged (no shape version bump). Verify:
      the definition file contains that bootstrap, package list, rustup
      layout, and environment, and does not mention `sudo`.

- [x] 1.2 Build a SIF headlessly with
      `singularity build --force /tmp/builder-base.sif builders/builder-base/builder-base.def`
      and run the checks below via `singularity exec` (do not commit the
      SIF). Verify all of them pass:
      `/etc/os-release` has `ID=debian` and `VERSION_CODENAME=trixie`;
      `gcc`, `g++`, `clang`, `clang++`, `cmake`, `make`, `ld`, `curl`,
      `wget`, `python3`, and `git` each print a version; a trivial C program
      and a trivial C++ program each compile, link, and run with both GCC
      and Clang; `sudo` is not on `PATH`; creating a file under `/usr`
      fails; `rustup show` reports stable as default and `rustc --version`
      reports a stable release; `rustc -vV` host architecture matches the
      machine that built the image; `CARGO_HOME` is `$HOME/.cargo` and a
      `cargo fetch` stores the crate under that directory.
