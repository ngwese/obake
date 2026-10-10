# Spec Delta

## MODIFIED Requirements

### Requirement: Build outputs within the image repository

Every host-visible build or verification output directory SHALL live inside the
image repository, SHALL have a name beginning with an underscore, and SHALL be
ignored by version control. This includes staged build artifacts and harness
work or log trees produced by verification and developer-VM tooling.

#### Scenario: Outputs inside the repository

- **WHEN** the build runs from a clone of the image repository alone
- **THEN** every output directory is created inside that repository tree

#### Scenario: Underscore directories ignored

- **WHEN** output directories have been created by a build
- **THEN** version control ignores them

#### Scenario: Harness work and logs under underscore directories

- **WHEN** a verification harness runs with default output paths
- **THEN** its host-visible work directory and log files are created under an
  underscore-named directory inside the image repository

### Requirement: Host-owned build outputs

Files written from the build, verification, or developer-VM containers to the
host SHALL be owned by the user who invoked the tooling, so the user can read,
move, and remove them without elevated privileges. This includes staged build
artifacts and host-visible trees under the image repository's underscore
output directories used for verification and developer-VM work. The tooling
SHALL also reclaim known leftover foreign-owned verification scratch that
predated underscore defaults so the invoking user can remove it without
elevated privileges.

#### Scenario: Container outputs owned by the invoking user

- **WHEN** a containerized build completes
- **THEN** the staged files on the host are owned by the invoking user

#### Scenario: Cleanup needs no elevated privileges

- **WHEN** the invoking user removes a completed build's, verification run's,
  or developer-VM output directory
- **THEN** the removal succeeds without elevated privileges

#### Scenario: Containerized verification outputs owned by the invoking user

- **WHEN** a harness runs through the verification environment container and
  writes under the verification or developer-VM output directories
- **THEN** those host-visible files are owned by the invoking user

#### Scenario: Legacy harness scratch reclaimable without elevated privileges

- **WHEN** foreign-owned leftover files exist at known former harness default
  paths beside the verification scripts
- **THEN** the ownership reclaim path makes those files owned by the invoking
  user so removal succeeds without elevated privileges

### Requirement: Verification environment runs as the invoking user

The containerized verification environment SHALL run as the invoking user's
uid and gid, and when the host exposes `/dev/kvm` or `/var/run/docker.sock`
SHALL add the host groups that own those paths so accelerated QEMU and nested
build invocations keep working. Operator documentation SHALL state these host
access prerequisites for containerized verification, including the Docker
Buildx CLI plugin required to build the environment images.

#### Scenario: Verify-env identity matches the invoker

- **WHEN** an operator runs a harness through the verification environment
  wrapper
- **THEN** processes inside the container run as the invoker's uid and gid

#### Scenario: KVM group added when the device exists

- **WHEN** `/dev/kvm` exists on the host and the verification environment
  wrapper starts
- **THEN** the container is granted the host group that owns `/dev/kvm`

#### Scenario: Docker socket group added when the socket exists

- **WHEN** `/var/run/docker.sock` exists on the host and the verification
  environment wrapper starts
- **THEN** the container is granted the host group that owns the socket

#### Scenario: Prerequisites documented

- **WHEN** an operator reads the verification or developer QEMU documentation
- **THEN** the docs state that containerized verification needs the Docker
  Buildx CLI plugin, access to the Docker socket, and, for acceleration, to
  `/dev/kvm`

## ADDED Requirements

### Requirement: Environment images built with Docker Buildx

The build-environment and verification-environment wrappers SHALL build their
container images with Docker Buildx (BuildKit). When the Buildx CLI plugin is
unavailable they SHALL fail immediately with an error that names the missing
dependency and how to install it, and SHALL NOT fall back to the legacy
Docker builder.

#### Scenario: Env image build uses Buildx

- **WHEN** an operator invokes the build-environment or verification-
  environment wrapper and the env image must be (re)built
- **THEN** the wrapper builds that image through Docker Buildx

#### Scenario: Missing Buildx fails loudly

- **WHEN** the Buildx CLI plugin is not available on the host
- **THEN** the wrapper exits non-zero with a message that identifies the
  missing Buildx dependency before attempting a legacy `docker build`
