# shape-jack-clients Specification

## Purpose

Lets a Singularity shape's JACK application use the host PipeWire graph by
shipping a Trixie PipeWire client library and connecting to the host socket.

## Requirements

### Requirement: Shape author guide

The repository SHALL include a guide that tells a shape author how to build
a Debian Trixie Singularity image for a JACK application and how to run that
image as `obake` against the host PipeWire socket.

#### Scenario: The guide covers build and run

- **WHEN** an author reads the guide
- **THEN** it names the Trixie base images, the compile-time JACK headers,
  the runtime `pipewire-jack` package, the `pw-jack` launch wrapper, and the
  Singularity bind of the `obake` user runtime directory

#### Scenario: The guide is linked from the shape index

- **WHEN** an author reads `shapes/README.md`
- **THEN** it links to the guide

### Requirement: ChucK and RNBO runner are Trixie PipeWire clients

`shapes/chuck/chuck.def` and `shapes/rnbo-runner/rnbo-runner.def` SHALL build
from Debian Trixie. Their runtime images SHALL load PipeWire's `libjack.so.0`
when the application starts. The ChucK application version pin SHALL stay at
`{{ chuck_version }}`. The RNBO runner pin SHALL be 1.4.5-9 with SDK tarball
`rnbo-src-1.4.5.tgz`.

#### Scenario: ChucK starts through the PipeWire JACK library

- **WHEN** the ChucK image is built and `pw-jack` launches `chuck`
- **THEN** the process loads PipeWire's `libjack.so.0` rather than jackd2's

#### Scenario: RNBO runner starts through the PipeWire JACK library

- **WHEN** the RNBO runner image is built and `pw-jack` launches
  `rnbooscquery`
- **THEN** the process loads PipeWire's `libjack.so.0` rather than jackd2's

#### Scenario: Application pins are unchanged

- **WHEN** the two definition files are read
- **THEN** the ChucK pin is still the `chuck_version` template and the RNBO
  runner pin is 1.4.5-9

### Requirement: Containers do not run an audio server

The ChucK and RNBO runner images SHALL NOT start `jackd`, `jackdbus`, or a
PipeWire daemon. Audio and JACK MIDI SHALL go to the host PipeWire socket.

#### Scenario: No audio server is the container entrypoint

- **WHEN** either image is started with its runscript
- **THEN** the entrypoint is the application under `pw-jack`, and no JACK or
  PipeWire daemon is started inside the container

### Requirement: Run binds the host user runtime directory

Running either image as `obake` SHALL bind the host's `/run/user/<obake-uid>`
into the container at the same path and SHALL set `XDG_RUNTIME_DIR` to that
path, so the PipeWire client opens the host socket.

#### Scenario: The documented run command binds the runtime directory

- **WHEN** an operator follows the guide's run command as `obake`
- **THEN** the container sees `/run/user/<obake-uid>` from the host and
  `XDG_RUNTIME_DIR` points at it
