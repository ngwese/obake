# Spec Delta: obake-session

## Purpose

Rig and session control for the obake host, including HTTP transfer of
session work-area files between the operator client and the daemon.

## ADDED Requirements

### Requirement: Session file HTTP transfers stream without a body-size cap

HTTP GET and PUT of session work-area bytes SHALL transfer the body as a
sequence of parts so the service need not hold the whole file in memory.
The service SHALL NOT reject a file transfer solely because the body
exceeds a small fixed buffer limit. PUT SHALL still create or replace a
regular file atomically and SHALL NOT leave a partial replace on failure.

#### Scenario: A multi-megabyte file round-trips without a body-size reject

- **WHEN** a client PUTs a regular file larger than 8 MiB into a session
  work area and then GETs that path
- **THEN** the PUT is accepted, the GET body matches the uploaded bytes,
  and neither request fails with a payload-too-large body-limit error

#### Scenario: A failed PUT leaves the prior file intact

- **WHEN** a PUT of a large file fails after some bytes have been
  received
- **THEN** any previously existing file at that path is unchanged

### Requirement: The CLI streams file get and put

`obake file get` and `obake file put` SHALL stream file octets in parts
between disk (or stdout) and HTTP so the CLI need not hold the whole
file in memory. Destinations `.` and `-`, and `--session`, SHALL keep
their existing meaning.

#### Scenario: File put accepts a multi-megabyte local file

- **WHEN** the operator runs `obake file put` with a local file larger
  than 8 MiB
- **THEN** the command completes successfully and the remote work-area
  path holds the same bytes

#### Scenario: File get streams to stdout

- **WHEN** the operator runs `obake file get large.bin -`
- **THEN** the raw bytes are written to stdout as they arrive
