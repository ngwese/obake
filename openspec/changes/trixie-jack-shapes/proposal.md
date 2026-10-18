# Proposal

## Why

`host-pipewire` removes the JACK server. Shapes built on Debian Bookworm link
`libjack-jackd2-0`, which speaks the classic JACK socket and cannot open a
PipeWire graph. A Trixie image carries PipeWire 1.4.2 client libraries, which
connect to the host's PipeWire 1.6.9 daemon, including JACK MIDI.

## What Changes

- Add a shape-author guide for building and running a Debian Trixie
  Singularity image whose JACK application loads PipeWire's `libjack.so.0`
  and connects to the host socket.
- **BREAKING**: Move `shapes/chuck/chuck.def` and
  `shapes/rnbo-runner/rnbo-runner.def` from Bookworm to Trixie. Their runtime
  images install `pipewire-jack` and start the application through `pw-jack`.
  They do not start a JACK or PipeWire daemon inside the container.
- Leave the ChucK and RNBO application version pins unchanged. `mod-host` and
  `siren` stay on Bookworm; the guide is the path for updating them later.
- Depends on `host-pipewire` for an end-to-end audio run. The images
  themselves can be built before that change is applied.

## Capabilities

### New Capabilities

- `shape-jack-clients`: how a Singularity shape's JACK application reaches
  the host PipeWire graph, including the author guide and the ChucK and
  RNBO runner images.

### Modified Capabilities

None.

## Impact

- New `shapes/docs/pipewire-jack.md`, linked from `shapes/README.md`.
- `shapes/chuck/chuck.def` and `shapes/rnbo-runner/rnbo-runner.def`.
- A pointer in `image/docs/user-guide.md` from the shape paragraph to that
  guide. No change to persistent host configuration or OTA. A shape built
  this way runs on a host that exposes a PipeWire socket new enough for a
  1.4.2 client, which is the socket `host-pipewire` provides. It does not
  run against a `jackd` host.
