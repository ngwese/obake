# Spec Delta

## ADDED Requirements

### Requirement: Singularity build temporary directory checked after boot

After a successful boot, verification SHALL confirm that `/tmp` is a tmpfs
mounted `nosuid` and not `nodev` or `noexec`, and that `obake` can
`singularity build --fakeroot` from the embedded local SIF without a
temporary-directory override and without a warning that `nodev` is set on
`/tmp`.

#### Scenario: /tmp mount flags in the guest

- **WHEN** the installed host has booted in QEMU and guest functional checks
  run
- **THEN** the harness confirms `/tmp` is tmpfs, includes `nosuid`, and does
  not include `nodev` or `noexec`

#### Scenario: Local fakeroot build does not warn

- **WHEN** guest functional checks run `singularity build --fakeroot` from
  the embedded local SIF as `obake`, with no temporary-directory override
- **THEN** the build completes and its output does not contain the warning
  that `nodev` is set on `/tmp`
