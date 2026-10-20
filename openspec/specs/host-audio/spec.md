# host-audio Specification

## Purpose

Provides the headless audio and MIDI runtime for the `obake` user: one
PipeWire graph that follows attached ALSA devices, schedules realtime work,
and is reachable from processes running as `obake`.

## Requirements

### Requirement: PipeWire runs for the obake user at boot

The image SHALL run PipeWire and its session manager as user services of the
`obake` account. Those services SHALL be active after boot without an
interactive login.

#### Scenario: Services are active after boot

- **WHEN** the host has finished boot and no one has logged in
- **THEN** the PipeWire daemon and its session manager are active in the
  `obake` user manager

### Requirement: No JACK server

The image SHALL NOT install or start `jackd` or `jackdbus`, and SHALL NOT
ship per-device JACK profiles or a `jack@.service` unit.

#### Scenario: jackd is absent

- **WHEN** a booted image is inspected
- **THEN** `jackd` is not on the system and no JACK server unit is active

### Requirement: Fixed low-latency clock

The PipeWire graph SHALL run at 48000 Hz with a quantum of 128 frames, and
SHALL keep that quantum for the life of the graph. Idle ALSA devices SHALL
NOT be suspended.

#### Scenario: Clock stays at the pinned quantum

- **WHEN** the PipeWire daemon is running
- **THEN** its graph rate is 48000 Hz and its quantum is 128 frames

#### Scenario: An idle device stays open

- **WHEN** an ALSA device has been opened by the session manager and no
  client is using it
- **THEN** the session manager does not suspend that device

### Requirement: ADAT, MixPre, and USBPre share that clock

The session manager SHALL open the ADAT card (`hw:CARD=ADAT`), the MixPre
card (`hw:CARD=II`), and the USBPre card (`hw:CARD=USBPre2`) at the graph
rate and quantum, with three ALSA periods. The factory configuration SHALL
NOT include an AES67 or RAVENNA profile.

#### Scenario: Each of those cards opens at 128 frames

- **WHEN** the ADAT, MixPre, or USBPre card is present while the session
  manager is running
- **THEN** that card is opened at 48000 Hz with a period of 128 frames and
  three periods

#### Scenario: No AES67 profile is installed

- **WHEN** the factory audio configuration is inspected
- **THEN** it contains no profile for `hw:CARD=RAVENNA`

### Requirement: Realtime scheduling without rtkit

PipeWire and WirePlumber data threads SHALL be allowed realtime priority 95
and unlimited locked memory through their `obake` user service limits. Factory
configuration SHALL disable RTKit and the realtime portal for
`libpipewire-module-rt`. The image SHALL NOT install rtkit. Interactive login
sessions SHALL obtain matching `@audio` limits through PAM as specified
separately; unit `Limit*` drop-ins remain the grant for linger-started audio
services.

#### Scenario: The user service grants realtime limits

- **WHEN** an operator inspects the PipeWire and WirePlumber user services
- **THEN** each sets realtime priority 95, unlimited realtime CPU time, and
  unlimited locked memory

#### Scenario: Factory config does not use RTKit

- **WHEN** the factory PipeWire and WirePlumber configuration is inspected
- **THEN** each disables RTKit and the realtime portal for the realtime
  module

#### Scenario: rtkit is not installed

- **WHEN** a booted image is inspected
- **THEN** no rtkit daemon is present

### Requirement: Login sessions inherit @audio realtime limits

An interactive login as `obake` (including SSH) SHALL apply the `@audio`
resource limits from `/etc/security/limits.d` through `pam_limits`, so the
login shell and its children may use realtime priority 95 and unlimited
locked memory. The image SHALL keep that `limits.d` file as the login-path
source of those limits.

#### Scenario: SSH login has realtime priority

- **WHEN** `obake` opens an interactive SSH login shell after boot
- **THEN** that shell reports a realtime priority limit of 95

#### Scenario: SSH login has unlimited memlock

- **WHEN** `obake` opens an interactive SSH login shell after boot
- **THEN** that shell reports an unlimited locked-memory limit

#### Scenario: limits.d file is installed

- **WHEN** a built image is inspected
- **THEN** `/etc/security/limits.d` contains the `@audio` rtprio and memlock
  entries used by PAM

### Requirement: ALSA devices are adopted on attach

When an ALSA sound card appears, the session manager SHALL open it and add
it to the graph without an operator starting a per-device service.

#### Scenario: A card that appears is opened

- **WHEN** an ALSA sound card appears while the session manager is running
- **THEN** the session manager opens that card and exposes it on the graph

### Requirement: MixPre USB mode switch still runs

Attaching a Sound Devices MixPre 6 (USB vendor `0926`, product `0206`) SHALL
still run `usb_modeswitch` to select USB configuration 2 before the audio
device is enumerated.

#### Scenario: MixPre attach invokes the mode switch

- **WHEN** a MixPre 6 USB device with vendor `0926` and product `0206` is
  attached
- **THEN** udev runs `usb_modeswitch` for that device with configuration 2

### Requirement: USBPre attach does not start a JACK server

Attaching a Sound Devices USBPre 2 SHALL NOT start a JACK server. The card
SHALL be adopted by the session manager like any other ALSA device.

#### Scenario: USBPre attach has no JACK start rule

- **WHEN** the installed udev rules are listed
- **THEN** none of them starts a JACK unit for USB vendor `0926` product
  `0202`

### Requirement: MIDI is available on the ALSA sequencer

PipeWire SHALL expose its MIDI ports on the kernel ALSA sequencer. A process
running as `obake` that opens `/dev/snd/seq` SHALL see those ports.

#### Scenario: Sequencer clients see PipeWire MIDI ports

- **WHEN** a MIDI device is managed by PipeWire and a process running as
  `obake` opens `/dev/snd/seq`
- **THEN** the PipeWire MIDI ports are visible as ALSA sequencer ports

### Requirement: PipeWire socket in the obake runtime directory

The PipeWire socket SHALL be created under the `obake` user runtime
directory and SHALL be connectable by processes running as `obake`, including
a Singularity process that bind-mounts that runtime directory.

#### Scenario: The socket exists after boot

- **WHEN** the host has finished boot
- **THEN** the PipeWire socket exists in `/run/user/<obake-uid>/` and is
  accessible to uid `obake`

### Requirement: No PulseAudio compatibility

The image SHALL NOT run a PulseAudio-compatible PipeWire service and SHALL
NOT install PulseAudio client libraries for that purpose.

#### Scenario: No Pulse service

- **WHEN** the `obake` user manager is inspected after boot
- **THEN** no `pipewire-pulse` service is installed or active

### Requirement: Factory config on the image, overrides in the home directory

Factory PipeWire and session-manager configuration SHALL be installed with
the image. The `obake` user SHALL be able to override it with files under
`/home/obake/.config` without elevated privileges, and those overrides SHALL
survive boots, slot switches, and operating system updates.

#### Scenario: Home overrides persist across an update

- **WHEN** `obake` has written a PipeWire or session-manager override under
  `/home/obake/.config` and the host then applies an operating system update
- **THEN** the override is still present and the factory files come from the
  newly booted image

### Requirement: Pinned PipeWire and WirePlumber versions

The image build SHALL pin PipeWire 1.6.9 and WirePlumber 0.5.18, with source
hashes, and SHALL record both versions in the build's pinned-input set.

#### Scenario: The booted image reports the pinned versions

- **WHEN** an image built from those pins boots
- **THEN** the installed PipeWire version is 1.6.9 and the installed
  WirePlumber version is 0.5.18

#### Scenario: Pinned inputs name both versions

- **WHEN** a build emits `pinned-inputs.json`
- **THEN** that file records PipeWire 1.6.9 and WirePlumber 0.5.18
