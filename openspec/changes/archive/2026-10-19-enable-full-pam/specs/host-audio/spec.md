# Spec Delta

## ADDED Requirements

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

## MODIFIED Requirements

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
