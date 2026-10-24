# Tasks

## 1. PipeWire recipe

- [x] 1.1 In `image/os/buildroot/package/obake-pipewire/Config.in`, select
  `BR2_PACKAGE_LIBSNDFILE`, `BR2_PACKAGE_NCURSES`,
  `BR2_PACKAGE_NCURSES_WCHAR`, and `BR2_PACKAGE_READLINE`. Do not select
  flac, libvorbis, opus, or ffmpeg. Leave `OBAKE_PIPEWIRE_VERSION` at
  1.6.9 in `obake-pipewire.mk` and do not edit `shapes/pipewire-tools/`.
  Verify the four selects are present, those four packages are not
  selected, the version line is still 1.6.9, and the shape directory is
  unchanged.
- [x] 1.2 In `image/os/buildroot/package/obake-pipewire/obake-pipewire.mk`,
  add `libsndfile`, `ncurses`, and `readline` to
  `OBAKE_PIPEWIRE_DEPENDENCIES`, and set `-Dpw-cat=enabled`,
  `-Dsndfile=enabled`, and `-Dreadline=enabled`. Keep
  `-Dpw-cat-ffmpeg=disabled`, `-Dv4l2=disabled`, and
  `-Dpipewire-v4l2=disabled`. Verify the dependency list and those six
  options read that way.

## 2. Guest check

- [x] 2.1 Extend `check_pipewire_runtime` in
  `image/verification/guest/obake-verify.sh` so it requires `pw-top`,
  `pw-cat`, `spa-resample`, `pw-play`, `pw-record`, `pw-midiplay`,
  `pw-midirecord`, `pw-midi2play`, `pw-midi2record`, `pw-dsdplay`,
  `pw-encplay`, and `pw-sysex`, requires `libreadline.so` in
  `/usr/bin/pw-cli` via `grep -a`, and fails when `pw-v4l2` is on the
  path. Verify the function does not launch `pw-top` or `pw-cat`.
- [x] 2.2 Rebuild a host image that includes `obake-pipewire` and run the
  headless QEMU guest functional check that exercises
  `check_pipewire_runtime`. Verify it passes with the new commands
  present, `pw-cli` referencing libreadline, and `pw-v4l2` absent.
