# Tasks

## 1. Author guide

- [x] 1.1 Add `shapes/docs/pipewire-jack.md` covering the Trixie base images,
  `libjack-jackd2-dev` at build time, `pipewire-jack` and `pw-jack` at run
  time, the `/run/user/$(id -u)` bind and `XDG_RUNTIME_DIR` setting, the
  warning not to start a daemon in the container, and the ALSA PCM and ALSA
  sequencer notes from design.md. Link it from `shapes/README.md` and from
  the shape paragraph in `image/docs/user-guide.md`. Verify the guide's run
  command includes the bind and `pw-jack`, and that both indexes link to the
  file.

## 2. ChucK

- [x] 2.1 Move both stages of `shapes/chuck/chuck.def` to Debian Trixie, use
  Trixie package names (`libasound2t64` in place of `libasound2`), install
  `pipewire-jack` in the final stage, and set the runscript to
  `pw-jack /usr/local/bin/chuck`. Keep `{{ chuck_version }}` as the
  application pin. Do not install the `jackd2` server. Verify the definition
  names `debian:trixie` and `debian:trixie-slim`, the runscript starts with
  `pw-jack`, and `chuck_version` is still the only app-version pin.
- [x] 2.2 Build the ChucK image with `singularity build` and verify
  `singularity exec <image> pw-jack ldd /usr/local/bin/chuck` reports
  PipeWire's `libjack.so.0` and `libpipewire`. This is the headless check;
  it does not require a sound card.

## 3. RNBO runner

- [x] 3.1 Move both stages of `shapes/rnbo-runner/rnbo-runner.def` to Debian
  Trixie, pin the RNBO SDK/runner at 1.4.5 / 1.4.5-9 and
  `-DWITH_JACKSERVER=Off`, install `pipewire-jack` in the final stage, and
  set the runscript to `pw-jack /usr/bin/rnbooscquery`. Update only the
  package names the suite change breaks. Verify the definition names
  `debian:trixie`, the runscript starts with `pw-jack`, and the RNBO version
  in the file is 1.4.5-9.
- [x] 3.2 Build the RNBO runner image with `singularity build` and verify
  `singularity exec <image> pw-jack ldd /usr/bin/rnbooscquery` reports
  PipeWire's `libjack.so.0` and `libpipewire`. This is the headless check;
  it does not require a sound card.
