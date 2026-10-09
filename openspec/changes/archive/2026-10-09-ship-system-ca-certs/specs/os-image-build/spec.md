# Spec Delta

## ADDED Requirements

### Requirement: System CA trust store

The image SHALL include a system CA trust store that TLS clients on the host
(including Singularity and curl) use by default to verify server certificates
for public HTTPS endpoints, without requiring a package manager at runtime.

#### Scenario: Public HTTPS verifies with the system store

- **WHEN** an image is built and a host TLS client connects to a server whose
  certificate chains to a well-known public CA
- **THEN** certificate verification succeeds using the image's system trust
  store

#### Scenario: Trust store present without a package manager

- **WHEN** an image is built
- **THEN** the root filesystem contains the system CA trust store and no
  runtime package manager is required to obtain it

### Requirement: Build-time extra root CAs

The build SHALL accept zero or more additional root CA certificates supplied
at image build time and SHALL merge them into the system CA trust store so
TLS clients trust servers signed by those roots (for example a personal
container registry).

#### Scenario: Extra root CA trusts a private endpoint

- **WHEN** an image is built with an additional root CA certificate supplied
  to the build
- **THEN** a host TLS client verifies a server certificate signed by that
  root using the system trust store

#### Scenario: No extras leaves the default store

- **WHEN** an image is built without additional root CA certificates
- **THEN** the system trust store contains only the default public CA set
  and the build still succeeds
