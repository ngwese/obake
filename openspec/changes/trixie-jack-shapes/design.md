# Design

## Context

See proposal.md for motivation. Behavior that shapes the approach:

- `shapes/chuck/chuck.def` is a two-stage `debian:bookworm` build. The build
  stage compiles ChucK with `make linux-jack` against `libjack-jackd2-dev`.
  The final stage is `debian:bookworm-slim` and installs `libjack-jackd2-0`.
  The runscript is `/usr/local/bin/chuck`. The app pin is `{{ chuck_version }}`.
- `shapes/rnbo-runner/rnbo-runner.def` is the same Bookworm pair. CMake sets
  `-DWITH_JACKSERVER=Off`, so the runner is a JACK client and does not start
  `jackd`. The final stage still installs `libjack-jackd2-dev` and a large
  build toolchain. The packed version is `1.4.4-8`.
- PipeWire does not speak the classic JACK socket. The client process must
  load PipeWire's `libjack.so.0`, which talks to the host socket. That
  library is built against the container's glibc. The host glibc is 2.42;
  Bookworm's is 2.36, so a host library bind-mounted into these images will
  not load.
- Debian Trixie ships PipeWire 1.4.2. A 1.4.2 client against the host's
  1.6.9 server is the direction PipeWire supports, and the 1.6 server accepts
  the UMP MIDI that the 1.4 JACK library writes. Trixie renamed the ALSA
  runtime library to `libasound2t64`. `libjack-jackd2-dev` still exists and
  depends on `libjack-jackd2-0`. `pipewire-jack` depends on
  `libpipewire-0.3-0` and the `pipewire` package, which pulls `pw-jack`.
- `pw-jack` prepends `/usr/lib/<triplet>/pipewire-0.3/jack` to
  `LD_LIBRARY_PATH`. jackd2's `libjack.so.0` can remain installed; it is not
  the library the process loads.
- Singularity does not bind `/run/user` by default. The host socket lives at
  `$XDG_RUNTIME_DIR/pipewire-0`.

## Goals / Non-Goals

**Goals:**

- One guide a shape author can follow without reading the host recipe.
- ChucK and the RNBO runner actually follow it.
- The container stays a client. The daemon stays on the host.

**Non-Goals:**

- Updating `mod-host` or `siren`.
- Trimming the RNBO final stage's build toolchain. Only the base suite, the
  JACK client library, and package names the suite change breaks.
- Installing `pipewire-alsa` into these two images. Both applications are
  JACK clients. The guide still documents the ALSA PCM plugin for other
  shapes.
- A second PipeWire daemon inside the container.
- Bumping `chuck_version` or the RNBO `1.4.4-8` pin. Those name the
  application, not the distro client library.

## Decisions

### Guide at `shapes/docs/pipewire-jack.md`

Link it from `shapes/README.md`. Point at it from the shape paragraph in
`image/docs/user-guide.md`.

The guide states:

- Base images are `debian:trixie` and `debian:trixie-slim`. Bookworm's
  PipeWire 0.3.65 client is not the supported client for this host.
- Compile and link with `libjack-jackd2-dev`. The JACK headers are the API.
  Do not install the `jackd2` server package.
- The runtime package is `pipewire-jack`. The runscript is `pw-jack` in
  front of the application. `pw-jack` selects PipeWire's private
  `libjack.so.0`.
- The run command, as `obake`, bind-mounts `/run/user/$(id -u)` at the same
  path and sets `XDG_RUNTIME_DIR` to it. Do not start `pipewire` or
  `wireplumber` in the container; the pulled-in daemon package is unused.
- ALSA sequencer clients open `/dev/snd/seq`, which Singularity binds with
  `/dev`. They do not need `pipewire-jack`. ALSA PCM clients install
  `pipewire-alsa` and route the default PCM at PipeWire, and they need the
  same runtime-directory bind.
- `mod-host` and `siren` are not converted yet. Follow this guide to convert
  them.

### The two definitions

Both stages of each definition move to Trixie. ChucK's final stage replaces
`libasound2` with `libasound2t64` and `libjack-jackd2-0` with `pipewire-jack`
(the jackd2 library may remain if a dependency pulls it). The runscript
becomes `pw-jack /usr/local/bin/chuck`.

The RNBO final stage gains `pipewire-jack`. Its runscript becomes
`pw-jack /usr/bin/rnbooscquery`. Build-stage package names that Trixie
removed or renamed are updated until `singularity build` completes. CMake
stays `-DWITH_JACKSERVER=Off` and `-DWITH_DBUS=Off`.

`com.afofo.obake.shape-app-version` stays `{{ chuck_version }}` on ChucK.
The RNBO definition has no such label today; do not add a new app-version
pin as part of the suite move.

### What "compatible" means at runtime

The process loads `libpipewire-0.3.so.0` from the Trixie image and connects
to the socket in the bind-mounted runtime directory. Audio uses the JACK
float format, which 1.4.2 and 1.6.9 negotiate at the same interface
versions. JACK MIDI is UMP on the wire; the 1.6.9 host accepts it. ALSA
sequencer MIDI does not use this library.

## Risks / Trade-offs

- [Trixie package renames beyond libasound] → The RNBO build stage installs
  several Bookworm packages (`google-mock`, conan via pip). The task is to
  fix what the build reports, not to refresh the RNBO source pin.
- [`pipewire-jack` pulls a daemon package] → Nothing enables it. The
  runscript is `pw-jack` plus the application. A container start of
  `pipewire.service` would fight the host socket.
- [End-to-end audio needs the host change and a device] → Image build and
  `ldd` under `pw-jack` are the headless checks. Hearing audio is hardware,
  after `host-pipewire`.

## Migration Plan

1. Write the guide and update both definitions.
2. Build both images and confirm `pw-jack` resolves PipeWire's `libjack`.
3. After `host-pipewire` is on a machine, run the documented command with a
   real device.

Rollback for a shape is rebuilding the previous Bookworm definition. That
image works with a slot that still runs `jackd`, and does not work with a
PipeWire host.

## Open Questions

None.
