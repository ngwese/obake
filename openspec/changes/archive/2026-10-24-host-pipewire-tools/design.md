# Design

## Context

See proposal.md for motivation. Constraints that shape the approach:

- `obake-pipewire` builds PipeWire 1.6.9. `src/tools/meson.build` always
  installs the graph tools already on the image (`pw-cli`, `pw-dump`,
  `pw-link`, and the rest). `pw-top` is compiled only when `ncursesw` is
  found. `pw-cat` and its nine names are compiled only when `-Dpw-cat` is
  allowed and libsndfile is found. `spa-resample` is compiled only when
  libsndfile is found.
- The recipe sets `-Dpw-cat=disabled`, `-Dsndfile=disabled`,
  `-Dreadline=disabled`, `-Dpw-cat-ffmpeg=disabled`, `-Dv4l2=disabled`, and
  `-Dpipewire-v4l2=disabled`.
- Bash already selects ncurses and readline. Wide-char ncurses is off, so
  staging has `ncurses.pc` and PipeWire's `dependency('ncursesw')` misses
  it. `pw-top` is then omitted with no configure error.
- Buildroot's libsndfile 1.2.2 enables external codecs only when flac,
  libvorbis, and opus are all selected. None of those are selected today.
  `--disable-external-libs` leaves the builtin formats (WAV, AIFF, AU, CAF,
  and the other internal ones).
- The filter-graph SPA plugin is already installed. Its link line includes
  `sndfile_dep`, which is empty until libsndfile is found.
- The guest image has no `ldd`, `readelf`, or `objdump`. BusyBox `grep -a`
  can see `DT_NEEDED` strings inside a binary.
- `shapes/pipewire-tools/` installs Alpine `pipewire-tools` and does not
  install `pipewire-spa-tools`. That shape is unchanged.

## Goals / Non-Goals

**Goals:**

- The same PipeWire 1.6.9 build produces `pw-top`, `pw-cat` and its names,
  readline `pw-cli`, and `spa-resample`.
- libsndfile is a dependency of `obake-pipewire`, staged for that build and
  installed on the target, with builtin formats only.
- The guest check fails when a required command is missing, when `pw-cli`
  does not reference libreadline, or when `pw-v4l2` is present.

**Non-Goals:**

- FFmpeg, FLAC, Vorbis, Opus, `pw-v4l2`, or the V4L2 camera plugin.
- A new PipeWire module that opens audio files. The library is there for
  later modules; this change does not add one.
- Removing or rebuilding `shapes/pipewire-tools/`.
- Bumping the PipeWire or WirePlumber pin.

## Decisions

### Enable the tools by selecting their libraries

In `image/os/buildroot/package/obake-pipewire/Config.in`, select
`BR2_PACKAGE_LIBSNDFILE`, `BR2_PACKAGE_NCURSES`,
`BR2_PACKAGE_NCURSES_WCHAR`, and `BR2_PACKAGE_READLINE`. Add `libsndfile`,
`ncurses`, and `readline` to `OBAKE_PIPEWIRE_DEPENDENCIES` so they are in
staging before Meson runs.

Set `-Dpw-cat=enabled`, `-Dsndfile=enabled`, and `-Dreadline=enabled`.
Leave `-Dpw-cat-ffmpeg=disabled`, `-Dv4l2=disabled`, and
`-Dpipewire-v4l2=disabled`. Do not select flac, libvorbis, opus, or ffmpeg.

`pw-top` and `spa-resample` have no Meson switches. They appear because
`ncursesw` and libsndfile are found. `-Dpw-cat=enabled` and
`-Dreadline=enabled` fail configure when their libraries are missing, which
is the failure we want. `pw-top` does not, so the guest check is what
catches a silent skip.

Alternative considered: install the Alpine shape's binaries onto the
rootfs. Rejected because later host modules need libsndfile linked into
this PipeWire build, and the tools should match the host daemon's glibc
build.

### Wide-char ncurses, not a second curses library

PipeWire 1.6 looks up `ncursesw` only. Buildroot's wide-char option builds
`libncursesw` and points `libncurses` at it. Bash and readline already pull
ncurses onto the image, so this flips the existing library to wide-char
instead of adding a new one. Terminfo entries for `xterm`, `screen`,
`linux`, and `putty` are already installed, which is what `pw-top` needs
over SSH.

### Guest check reads the binary, and does not run the tools

Extend `check_pipewire_runtime` in
`image/verification/guest/obake-verify.sh`. Require `command -v` for
`pw-top`, `pw-cat`, `spa-resample`, and the nine `pw-cat` names. Fail if
`pw-v4l2` is on the path. Fail unless `grep -a` finds `libreadline.so` in
`/usr/bin/pw-cli`. Do not start `pw-top` or `pw-cat`; both can require a
terminal or a running graph, and this check must pass with no audio device.

No host documentation names these tools as absent, and the shape README
still describes a portable client. No doc edit.

## Risks / Trade-offs

- [pw-top is skipped when `ncursesw` is missing at configure] → Select
  wide-char ncurses, depend on it, and fail the guest check if `pw-top` is
  absent.
- [Wide-char ncurses changes the curses ABI] → Buildroot rebuilds the
  packages that link it. The image does not keep a non-wide `libncurses`
  beside `libncursesw`.
- [A later selection of flac, libvorbis, and opus together enables
  libsndfile's external codecs] → This change selects none of them. The
  guest requirement is the installed library's dynamic dependencies, so
  that flip would fail the codec check.
- [filter-graph starts linking libsndfile] → The plugin can open audio
  files. That is the library later modules need. The rootfs grows by
  libsndfile and the new binaries.
- [No `ldd` on the guest] → Match the `libreadline.so` string in the
  binary. A renamed SONAME would need the check updated with the pin.

## Migration Plan

1. Land the recipe change and the guest check.
2. Rebuild the image. The PipeWire 1.6.9 tarball does not change, so the
   vendor download cache stays as it is.
3. OTA or reinstall picks up the new slot. Rollback is the previous slot.
   Nothing under `/home/obake` changes.

## Open Questions

None.
