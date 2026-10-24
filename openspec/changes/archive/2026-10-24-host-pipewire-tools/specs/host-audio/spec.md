# Spec Delta

## ADDED Requirements

### Requirement: PipeWire audio tools on the image
The image SHALL provide `pw-top`, `pw-cat`, and `spa-resample` as commands.
`pw-cat` SHALL also be available as `pw-play`, `pw-record`, `pw-midiplay`,
`pw-midirecord`, `pw-midi2play`, `pw-midi2record`, `pw-dsdplay`,
`pw-encplay`, and `pw-sysex`. The image SHALL NOT provide `pw-v4l2`.

#### Scenario: Audio tools are on the default path
- **WHEN** a built image is inspected
- **THEN** `pw-top`, `pw-cat`, `spa-resample`, `pw-play`, `pw-record`,
  `pw-midiplay`, `pw-midirecord`, `pw-midi2play`, `pw-midi2record`,
  `pw-dsdplay`, `pw-encplay`, and `pw-sysex` are executable commands
- **AND** `pw-v4l2` is not installed

### Requirement: pw-cli line editing
The installed `pw-cli` SHALL link libreadline so an interactive session can
edit command lines.

#### Scenario: pw-cli links readline
- **WHEN** the installed `pw-cli` binary is inspected
- **THEN** its dynamic dependencies include libreadline

### Requirement: Builtin-format libsndfile for host PipeWire
The image SHALL install libsndfile and make it available to the host
PipeWire build. That library SHALL expose only its builtin formats.
`pw-cat` SHALL NOT link FFmpeg, and libsndfile SHALL NOT link external
FLAC, Vorbis, or Opus libraries.

#### Scenario: pw-cat has no FFmpeg dependency
- **WHEN** the installed `pw-cat` binary is inspected
- **THEN** its dynamic dependencies do not include FFmpeg libraries

#### Scenario: libsndfile has no external audio codecs
- **WHEN** the installed libsndfile is inspected
- **THEN** its dynamic dependencies do not include FLAC, Vorbis, or Opus
