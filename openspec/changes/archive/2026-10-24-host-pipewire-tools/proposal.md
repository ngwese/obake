# Proposal

## Why

The host PipeWire graph can be inspected and exercised with `pw-top`,
`pw-cat`, and `spa-resample`, but those commands are absent from the base
image. Operators currently need the `pipewire-tools` shape for them, and
later host PipeWire modules need libsndfile on the image anyway.

## What Changes

- Install the missing audio PipeWire tools on the base image: `pw-top`,
  `pw-cat` (and its play, record, MIDI, DSD, encoded, and sysex names), and
  `spa-resample`. Build `pw-cli` with readline.
- Add libsndfile to the host PipeWire build with builtin formats only, so
  those tools and later host modules can open audio files. Do not add
  FFmpeg, FLAC, Vorbis, or Opus.
- Leave `pw-v4l2` and the V4L2 camera plugin out. This image stays audio
  focused.
- Keep the `pipewire-tools` shape as a portable client. It is unchanged.
- Confirm the new commands, and the continued absence of `pw-v4l2`, in the
  headless guest checks.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `host-audio`: the base image provides the audio PipeWire tools and a
  builtin-format libsndfile, and still omits `pw-v4l2` and FFmpeg.
- `image-verification`: after boot, the guest checks require those tools and
  require that `pw-v4l2` is absent.

## Impact

- `image/os/buildroot/package/obake-pipewire/` gains libsndfile, wide-char
  ncurses, and readline, and enables the corresponding PipeWire tools.
- The rootfs slot grows by libsndfile and a few binaries. An OTA or
  reinstall picks that up. No persistent configuration, factory audio
  config, or shape portability change.
- `image/verification/guest/obake-verify.sh` gains the tool presence checks.
