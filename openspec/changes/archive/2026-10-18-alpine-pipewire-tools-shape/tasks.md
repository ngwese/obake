# Tasks

## 1. Shape definition

- [x] 1.1 Add `shapes/pipewire-tools/pipewire-tools.def` as a single-stage
  Alpine Singularity definition that installs `pipewire-tools` via `apk`,
  sets `%runscript` to `exec "$@"`, does not start a PipeWire daemon, and
  sets `com.afofo.obake.shape-name` to `pipewire-tools` with
  `com.afofo.obake.shape-app-version` equal to the installed PipeWire
  package version (1.6.9 while Alpine ships that). Verify by reading the
  def: Alpine base, `apk add` of `pipewire-tools`, no daemon startscript,
  and the three shape labels present.
- [x] 1.2 Add `shapes/pipewire-tools/README.md` with build and run commands,
  including `--bind /run/user/$(id -u)` and `XDG_RUNTIME_DIR`. Add a
  `pipewire-tools` row to `shapes/README.md`. Verify those docs name
  `pw-cat`, `pw-top`, and `pw-profiler` and show the runtime-dir bind.

## 2. Headless verification

- [x] 2.1 Build the SIF with Singularity (`Bootstrap: docker`) and verify
  `singularity exec <sif> sh -c 'command -v pw-cat pw-play pw-record pw-top pw-profiler pw-cli'`
  succeeds for each tool. Verify the image labels report shape-name
  `pipewire-tools` and an app version matching the Alpine PipeWire package.
- [x] 2.2 On a host with an active `obake` PipeWire session, run
  `pw-cli info` (or equivalent) through the SIF with the runtime-dir bind
  and confirm it reaches remote `pipewire-0`. Verify no PipeWire daemon is
  started by the shape's runscript.
