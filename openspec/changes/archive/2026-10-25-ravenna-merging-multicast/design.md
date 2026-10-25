# Design

## Context

See proposal.md for motivation. Today `Config::resolve_source_destination`
reads `[multicast].address` and steps `base_port + 2 * index`. Qualify
already knows the PTP interface IPv4 as `source_ip` and holds sources
when it is missing. The main spec currently requires a shared group
under port stepping; this change adds an opt-in `merging` path beside
it. The name `merging` is deliberate so a later generic address-step
allocator can still use something like `address` without a rename.

## Goals / Non-Goals

**Goals:**

- Opt-in `allocation = "merging"` that builds
  `239.{session_base+index}.{o3}.{o4}` from the PTP unicast.
- Default and undocumented omission stay on `port`.
- Keep pins and `(group, port)` uniqueness.
- Document the layout and why `address` is reserved for a future mode.

**Non-Goals:**

- A generic "step any octet" allocator.
- Changing sink receive logic (sinks still follow peer SDP).
- Changing the image seed to `merging` by default.
- Inferring session numbers from Merging UI or mDNS TXT.

## Decisions

### Config keys

```toml
[multicast]
allocation = "port"       # default when omitted
# allocation = "merging"
session_base = 1          # used only by merging; default 1
base_port = 5004
address = "239.1.60.50"   # required for port; forbidden for merging
ttl = 15
```

- Reject unknown `allocation` values at load time.
- When `allocation = "merging"`, reject a set `[multicast].address` so
  operators do not think a static base still applies.
- When `allocation = "port"`, keep today's rule that `address` is
  required if any `[[source]]` exists.
- `session_base` ignored under `port` (allowed but unused) so toggling
  modes does not force deleting keys.

**Alternatives considered:** template strings (`239.{session}.{host3}…`)
— flexible but stringly; deferred. Always require `address` as a seed
octet — fights the Merging "follow unicast" story. Name the mode
`address` — burns the generic name; rejected per proposal.

### Resolve needs the PTP IPv4

`resolve_source_destination` today is pure config. Merging needs
`source_ip`. Thread an `Option<Ipv4Addr>` into resolve (or a thin
wrapper used from qualify) and treat `None` as destination failure for
unpinned merging sources so they land in the existing down path
(`bad-destination` or `interface-missing` — prefer the same reason
qualify already uses when the iface has no address).

**Alternatives considered:** resolve only at load into a static map —
wrong when DHCP renews the last octet. Snapshot at startup only —
same problem.

### Session octet

`session = session_base + index` as `u8`. Overflow or producing the SAP
group `239.255.255.255` holds that source down. Index remains the
order of `[[source]]` in the file, matching today's port step.

### Pins

Unchanged: source `address` / `port` override the allocator. Duplicate
`(group, port)` still holds both. A merging source that pins only
`address` still takes `base_port` (not a stepped port).

### Docs

REFERENCE_GUIDE and README describe both modes with a short diagram of
the Merging formula. Examples keep `port` as the worked sample and add
a commented `merging` block.

## Risks / Trade-offs

- [Host IP change moves groups] → intentional Merging mimicry; document
  it. Pins freeze a group when needed.
- [session_base + many sources overflows] → hold that source; do not
  wrap.
- [Operators set both `address` and `merging`] → hard reject at load.
- [Qualify reason naming for missing IP] → reuse existing
  `interface-missing` when possible so status stays familiar.

## Migration Plan

- Existing configs omit `allocation` → `port`, no behavior change.
- To opt in: set `allocation = "merging"`, remove `[multicast].address`,
  optionally set `session_base`, keep `base_port`.
- Rollback: set `allocation = "port"` (or remove it) and restore
  `address`.

## Open Questions

None that block specs or tasks. A later generic address-step mode can
be a separate change if pinning or `merging` is not enough.
