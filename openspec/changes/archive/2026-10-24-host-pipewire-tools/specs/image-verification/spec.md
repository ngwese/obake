# Spec Delta

## ADDED Requirements

### Requirement: PipeWire audio tools checked after boot
After a successful boot, verification SHALL confirm that `pw-top`, `pw-cat`,
`spa-resample`, and the `pw-cat` names `pw-play`, `pw-record`,
`pw-midiplay`, `pw-midirecord`, `pw-midi2play`, `pw-midi2record`,
`pw-dsdplay`, `pw-encplay`, and `pw-sysex` are installed, that `pw-cli`
links libreadline, and that `pw-v4l2` is absent. The check SHALL NOT
require an audio device.

#### Scenario: Guest reports the audio tools
- **WHEN** guest functional checks run after boot
- **THEN** the suite fails unless those commands are present, `pw-cli`
  links libreadline, and `pw-v4l2` is absent
