# Spec Delta

## ADDED Requirements

### Requirement: UMP MIDI is enabled on the ALSA sequencer bridge

Factory WirePlumber configuration SHALL enable UMP (MIDI 2.0 packet)
handling on the ALSA sequencer MIDI bridge by default so USB MIDI 2.0
devices and UMP-aware clients can use the host without a home-directory
override. An `obake` user override under `/home/obake/.config` SHALL still
be able to change that setting.

#### Scenario: Factory config requests UMP

- **WHEN** the installed WirePlumber factory drop-ins are inspected
- **THEN** the ALSA MIDI bridge is configured with `api.alsa.seq.ump`
  set to true

#### Scenario: Home override can disable UMP

- **WHEN** `obake` installs a WirePlumber override that sets
  `api.alsa.seq.ump` to false
- **THEN** that override takes effect for subsequent WirePlumber starts
  without elevating privileges

## MODIFIED Requirements

### Requirement: Pinned PipeWire and WirePlumber versions

The image build SHALL pin PipeWire 1.6.10 and WirePlumber 0.5.18, with
source hashes, and SHALL record both versions in the build's pinned-input
set.

#### Scenario: The booted image reports the pinned versions

- **WHEN** an image built from those pins boots
- **THEN** the installed PipeWire version is 1.6.10 and the installed
  WirePlumber version is 0.5.18

#### Scenario: Pinned inputs name both versions

- **WHEN** a build emits `pinned-inputs.json`
- **THEN** that file records PipeWire 1.6.10 and WirePlumber 0.5.18
