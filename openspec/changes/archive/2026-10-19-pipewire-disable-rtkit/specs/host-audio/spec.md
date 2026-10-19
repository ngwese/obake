# Spec Delta

## MODIFIED Requirements

### Requirement: Realtime scheduling without rtkit

PipeWire and WirePlumber data threads SHALL be allowed realtime priority 95
and unlimited locked memory through their `obake` user service limits. Factory
configuration SHALL disable RTKit and the realtime portal for
`libpipewire-module-rt`. The image SHALL NOT install rtkit.

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
