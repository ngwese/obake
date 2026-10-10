# Spec Delta

## ADDED Requirements

### Requirement: Singularity builds tolerate the default temporary directory

With no temporary-directory override, `singularity build --fakeroot` run by
`obake` SHALL complete a build from a local image, and SHALL NOT warn that
`nodev` is set on `/tmp`. The same absence of that warning SHALL hold for a
run that converts a non-local image into a SIF before starting it.

#### Scenario: Fakeroot build from a local image

- **WHEN** `obake` runs `singularity build --fakeroot` from a local image
  and does not set a temporary directory
- **THEN** the build completes and does not warn that `nodev` is set on
  `/tmp`

#### Scenario: Image conversion does not warn about nodev

- **WHEN** `obake` runs a container that Singularity must convert into a SIF
  first, without a temporary-directory override
- **THEN** the conversion does not warn that `nodev` is set on `/tmp`
