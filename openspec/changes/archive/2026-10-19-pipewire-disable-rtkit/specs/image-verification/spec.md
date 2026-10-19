# Spec Delta

## MODIFIED Requirements

### Requirement: PipeWire user runtime checked after boot

After a successful boot, verification SHALL confirm that the `obake` user
manager is running PipeWire and its session manager, that the PipeWire socket
exists in the `obake` runtime directory, that the PipeWire and WirePlumber
user services grant realtime priority 95, that `jackd` is absent, that no
PulseAudio-compatible PipeWire service is installed, and that the MixPre
`usb_modeswitch` udev rule is installed. The check SHALL NOT require an audio
device.

#### Scenario: Guest reports the PipeWire user runtime

- **WHEN** guest functional checks run after boot
- **THEN** the suite fails unless the PipeWire daemon and session manager are
  active for `obake`, the PipeWire socket exists under `/run/user/<obake-uid>/`,
  the PipeWire and WirePlumber user services grant realtime priority 95,
  `jackd` is absent, no `pipewire-pulse` unit is installed, and the MixPre
  udev rule is present
