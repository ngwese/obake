# Design

## Context

See proposal.md for motivation. Today `obake-pipewire` pins
`OBAKE_PIPEWIRE_VERSION = 1.6.9` with a matching `.hash`. WirePlumber is
0.5.18. Factory host config lives under `image/host/wireplumber/` and
`image/host/pipewire/`; ALSA audio rules are in
`wireplumber.conf.d/10-obake-alsa.conf` with no MIDI/UMP settings.
Kernel USB MIDI 2.0 is already required for qdl (`os-image-build`).

PipeWire 1.4 preferred UMP on the graph. In 1.6.3 most nodes returned to
MIDI 1.0 to avoid lossy Midi1↔UMP conversions; 1.7 documents that UMP on
the ALSA sequencer must be enabled explicitly. The WirePlumber property
is `api.alsa.seq.ump` under `monitor.alsa-midi.properties`.

## Goals / Non-Goals

**Goals:**

- Ship PipeWire 1.6.10 with refreshed source hash and docs.
- Factory-enable UMP on the ALSA MIDI bridge for all boards using the
  shared host WirePlumber drop-ins.
- Keep WirePlumber 0.5.18 unless configure/runtime forces a bump.
- Record the new PipeWire version in pinned inputs / version stamp.

**Non-Goals:**

- Migrating to PipeWire 1.7.x.
- Changing JACK MIDI port flags inside shapes.
- Forcing every client to speak UMP (legacy MIDI 1.0 clients should keep
  working via PipeWire conversion where the stack still converts).
- Reworking ALSA audio period rules.

## Decisions

### 1. Version bump only in `obake-pipewire`

Update `obake-pipewire.mk` / `.hash` to 1.6.10. Re-download and record
the tarball sha256 (and license digests if they change). Keep existing
Meson `CONF_OPTS` unless 1.6.10 renames an option (configure will fail
loudly).

### 2. Factory UMP via WirePlumber drop-in

Add `image/host/wireplumber/wireplumber.conf.d/10-obake-alsa-midi.conf`
(or extend the alsa drop-in) with:

```
monitor.alsa-midi.properties = {
  api.alsa.seq.ump = true
}
```

Prefer a dedicated MIDI drop-in so audio period rules stay separate.
Confirm at apply time against WirePlumber 0.5.18 / PipeWire 1.6.10 docs
that this key is accepted (alternate spellings in older trees were
rare).

### 3. Image VERSION bump

Bump `image/os/buildroot/VERSION` patch or minor so OTA artifacts are
distinct from 0.7.0 builds that still carry PipeWire 1.6.9. Exact
semver step chosen at apply (recommend patch if 0.7.0 already shipped
with session work only, or follow the operator’s usual minor cadence).

### 4. Verification

- Rebuild (or package-level fetch) to confirm hash/configure.
- Guest or device check: `pw-cli info` / package version string shows
  1.6.10; WirePlumber props or `pw-dump` show UMP enabled on the ALSA
  MIDI bridge after boot.
- Existing MIDI sequencer visibility requirement remains.

## Risks / Trade-offs

- **[Risk] UMP breaks a MIDI 1.0-only client** → Mitigation: PipeWire
  still converts for non-UMP consumers; allow home override to set
  `api.alsa.seq.ump = false`.
- **[Risk] Property name differs on 0.5.18** → Mitigation: verify
  against installed WirePlumber schema/docs during apply; adjust drop-in
  before merge.
- **[Risk] 1.6.10 tarball hash / patch drift** → Mitigation: recompute
  hash from the GitHub release tarball; re-evaluate
  `0001-rtp-sink-quantize-timestamps.patch` if present for apply cleanly.
- **[Trade-off] Not jumping to 1.7** → Stay on the 1.6 stable line the
  image already tracks.

## Migration Plan

1. Land pin + factory drop-in + docs + VERSION.
2. Rebuild image / `--update` bundles; deploy via existing OTA or
   `obake device update`.
3. Operators who disabled UMP locally keep their
   `/home/obake/.config` override across the update.

## Open Questions

None blocking. Confirm at apply whether VERSION should move 0.7.0 →
0.7.1 or 0.8.0.
