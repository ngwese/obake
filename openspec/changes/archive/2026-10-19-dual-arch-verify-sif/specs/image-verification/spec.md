# Spec Delta

## ADDED Requirements

### Requirement: Arch-matched Singularity verify fixture

When guest functional checks are installed (`VERIFY=1`), the verification
image SHALL embed a Singularity SIF whose CPU architecture matches the guest.
The repository SHALL provide separate fixtures for x86_64 and arm64 guests.
The embedded path in the guest SHALL remain `/opt/obake/verify.sif`.

#### Scenario: x86_64 VERIFY image embeds x86_64 fixture

- **WHEN** a `VERIFY=1` image is built for an x86_64 board
- **THEN** the embedded `/opt/obake/verify.sif` is the x86_64 fixture and
  Singularity on that guest accepts its architecture

#### Scenario: aarch64 VERIFY image embeds arm64 fixture

- **WHEN** a `VERIFY=1` image is built for an aarch64 board
- **THEN** the embedded `/opt/obake/verify.sif` is the arm64 fixture and
  Singularity on that guest accepts its architecture

#### Scenario: Fixtures are present for both architectures

- **WHEN** an operator inspects `image/verification/fixtures/`
- **THEN** both `verify-x86_64.sif` and `verify-arm64.sif` are present
