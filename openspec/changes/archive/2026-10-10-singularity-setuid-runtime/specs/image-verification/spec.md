# Spec Delta

## MODIFIED Requirements

### Requirement: Singularity run checked after boot

After a successful boot, the verification SHALL confirm that the `obake` user
can run a local Singularity SIF embedded in the verification image using the
setuid helper and kernel SquashFS mounts, so a FUSE-only or broken setuid
install fails headless verification before release.

#### Scenario: Local SIF run as obake

- **WHEN** the installed host has booted in QEMU and guest functional checks
  run
- **THEN** the harness confirms `singularity run` (or equivalent) of the
  embedded local SIF succeeds as `obake` and that the mount path is kernel
  SquashFS rather than squashfuse/FUSE

## ADDED Requirements

### Requirement: Singularity fakeroot checked after boot

After a successful boot, the verification SHALL confirm that the `obake` user
can use Singularity `--fakeroot` against the embedded local SIF, so missing
subordinate ID maps or a broken setuid fakeroot path fails headless
verification before release.

#### Scenario: Fakeroot exec as obake

- **WHEN** the installed host has booted in QEMU and guest functional checks
  run
- **THEN** the harness confirms `singularity exec --fakeroot` of the embedded
  local SIF reports uid `0` inside the container as `obake`
