# Spec Delta

## MODIFIED Requirements

### Requirement: Host-owned build outputs

Files written from the build, verification, or developer-VM containers to the
host SHALL be owned by the user who invoked the tooling, so the user can read,
move, and remove them without elevated privileges. This includes staged build
artifacts and host-visible trees under the image repository's underscore
output directories used for verification and developer-VM work.

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

## ADDED Requirements

### Requirement: Verification environment runs as the invoking user

The containerized verification environment SHALL run as the invoking user's
uid and gid, and when the host exposes `/dev/kvm` or `/var/run/docker.sock`
SHALL add the host groups that own those paths so accelerated QEMU and nested
build invocations keep working. Operator documentation SHALL state these host
access prerequisites for containerized verification.

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
- **THEN** the docs state that containerized verification needs access to the
  Docker socket and, for acceleration, to `/dev/kvm`
