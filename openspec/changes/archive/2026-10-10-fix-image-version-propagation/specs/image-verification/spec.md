# Spec Delta

## ADDED Requirements

### Requirement: Artifact pinned-inputs version matches VERSION

After a build stages artifacts, verification SHALL confirm that
`pinned-inputs.json` `image_version` equals the staged `VERSION` stamp, so a
mismatched image-version record fails headlessly without requiring a guest
boot.

#### Scenario: Matching version stamps

- **WHEN** a completed artifact directory is checked
- **THEN** `pinned-inputs.json` `image_version` equals the contents of
  `VERSION`

#### Scenario: Mismatch fails

- **WHEN** `pinned-inputs.json` `image_version` differs from `VERSION`
- **THEN** the check fails
