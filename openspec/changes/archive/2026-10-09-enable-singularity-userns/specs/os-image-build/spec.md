# Spec Delta

## ADDED Requirements

### Requirement: Kernel namespaces for container runtime

Every board kernel SHALL enable Linux namespaces including user, PID, UTS,
IPC, network, and cgroup namespaces, so the non-setuid Singularity runtime can
create the namespaces it needs to mount and enter containers.

#### Scenario: User namespace supported

- **WHEN** a built image boots
- **THEN** creating a user namespace succeeds for an unprivileged process
  (for example the `obake` login user)

#### Scenario: Companion namespaces enabled

- **WHEN** a board kernel is built with the shared host fragment
- **THEN** PID, UTS, IPC, network, and cgroup namespaces are available in
  addition to user namespaces

### Requirement: Kernel cgroup controllers for container runtime

Every board kernel SHALL enable cgroup support with the common resource
controllers needed by container workloads (memory, CPU accounting, cpusets,
devices, and PIDs), without enabling realtime group scheduling that conflicts
with the project's PREEMPT_RT configuration.

#### Scenario: Core cgroup controllers present

- **WHEN** a built image boots
- **THEN** cgroup controllers for memory, CPU accounting, cpusets, devices,
  and PIDs are available

#### Scenario: Realtime group scheduling omitted

- **WHEN** a board kernel is built with the shared host and realtime fragments
- **THEN** realtime group scheduling is not enabled

### Requirement: Singularity runnable as host login user

The image SHALL allow the `obake` login user to run a Singularity container
from a local SIF without requiring a setuid helper or a root login, using the
kernel namespace and FUSE support already required by the non-setuid runtime.

#### Scenario: Local SIF runs as obake

- **WHEN** the `obake` user runs `singularity run` (or equivalent) against a
  local SIF on a booted image
- **THEN** the container starts successfully without a user-namespace error
