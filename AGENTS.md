# AGENTS.md

## Project

obake is a headless Linux server for running user-defined audio effects and
synthesis processes, plus sequencers, MIDI processors, and generative control
systems for MIDI and OSC. Core capabilities: fully headless operation,
persistent configuration across boot cycles, OTA upgrades of the base system,
and portability of configurations between hosts via Singularity containerized
workloads ("shapes").

- `shapes/<name>/<name>.def` — Singularity definition files for each workload.
- `host/` — host-level udev rules and JACK/systemd configuration.

## Spec-driven workflow (OpenSpec)

This project uses [OpenSpec](https://openspec.dev) for spec-driven development.
Before writing code for a non-trivial change, plan it with OpenSpec.

- Plan a change: `/opsx-propose` (or the `openspec-propose` skill). This creates
  a proposal, spec deltas, design, and tasks under `openspec/changes/`.
- Implement a planned change: `/opsx-apply`.
- Archive a completed change: `/opsx-archive`.
- Other workflows: `/opsx-explore`, `/opsx-update`, `/opsx-sync`.

Read `openspec/config.yaml` for project context and artifact rules. Do not
implement a change until its planning artifacts are complete and the user has
asked you to apply it.

## Commits

Follow [Conventional Commits](https://www.conventionalcommits.org/). Every line
of a commit message — the subject and every body line — MUST be **less than 80
characters**. Any line that would exceed this limit MUST be hard wrapped.

Format: `<type>(<optional scope>): <description>`

Common types: `feat`, `fix`, `chore`, `docs`, `refactor`, `test`, `build`,
`ci`, `perf`. Scope is often a shape name (e.g. `chuck`, `rnbo-runner`) or
`host`.

Examples:

```
feat(chuck): bump pinned chuck to 1.5.5.5
fix(host): increase aes67 periods to 6
```

Only commit when the user explicitly asks.
