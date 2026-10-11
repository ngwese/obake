# builder-base Specification

## Purpose

An immutable Singularity image, based on Debian trixie, that provides C, C++,
and Rust compilers plus common native build tools.

## Requirements

### Requirement: Debian trixie base

The built builder-base image SHALL identify as Debian trixie.

#### Scenario: OS release identity

- **WHEN** `/etc/os-release` is read inside the built image
- **THEN** `ID` is `debian` and `VERSION_CODENAME` is `trixie`

### Requirement: Native build tools on PATH

The image SHALL provide `gcc`, `g++`, `clang`, `clang++`, `cmake`, `make`,
`ld`, `curl`, `wget`, `python3`, and `git` on `PATH`, and SHALL compile and
link C and C++ programs with both GCC and Clang.

#### Scenario: Tool versions

- **WHEN** `gcc`, `g++`, `clang`, `clang++`, `cmake`, `make`, `ld`, `curl`,
  `wget`, `python3`, and `git` are each invoked with a version flag inside
  the image
- **THEN** each command exits successfully

#### Scenario: C and C++ link with GCC and Clang

- **WHEN** a trivial C program and a trivial C++ program are compiled and
  linked inside the image with `gcc`, `g++`, `clang`, and `clang++`
- **THEN** each resulting binary runs inside the image

### Requirement: Immutable image

The built artifact SHALL be a read-only SIF. The image SHALL NOT provide
`sudo` and SHALL NOT allow the invoking user to install distribution packages
into the image.

#### Scenario: sudo is absent

- **WHEN** the image is entered as the invoking user
- **THEN** `sudo` is not found on `PATH`

#### Scenario: Image root is not writable

- **WHEN** a process inside the built SIF attempts to create a file under
  `/usr`
- **THEN** the write fails

### Requirement: Stable Rust toolchain for the build host

The image SHALL provide `rustup`, `rustc`, and `cargo` on `PATH`. The default
toolchain SHALL be the stable channel whose host architecture matches the
machine that built the image.

#### Scenario: Default toolchain is stable

- **WHEN** `rustup show` and `rustc --version` are run inside the image
- **THEN** the default toolchain is stable and `rustc` reports a stable
  release

#### Scenario: Toolchain architecture matches the build machine

- **WHEN** `rustc -vV` is run inside the image
- **THEN** the reported host triple uses the CPU architecture of the machine
  that built the image

### Requirement: Cargo cache in the user home

The image SHALL set the default `CARGO_HOME` to `.cargo` under the invoking
user's home directory, so Cargo can download crates without writing to the
image.

#### Scenario: Crate cache lands in the home directory

- **WHEN** `cargo` fetches a crate inside the image as the invoking user
- **THEN** the download is stored under that user's home `.cargo` directory
  and the SIF contents are unchanged
