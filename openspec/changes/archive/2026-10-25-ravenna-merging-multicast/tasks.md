# Tasks

## 1. Config and destination resolve

- [x] 1.1 Add `allocation` (`port` | `merging`, default `port`) and
      `session_base` (default `1`) to `[multicast]` in
      `services/obake-ravenna/src/config.rs`; reject unknown
      `allocation` and reject `[multicast].address` when
      `allocation = "merging"`; verify load tests cover both rejects
      and that omitted `allocation` still parses as `port`
- [x] 1.2 Extend `resolve_source_destination` to take the PTP interface
      IPv4 (`Option<Ipv4Addr>`) and implement merging groups
      `239.{session_base+index}.{o3}.{o4}` with shared `base_port`;
      verify unit tests match `192.168.60.50` → `239.1.60.50:5004` /
      `239.2.60.50:5004`, hold on missing IPv4 or session overflow, and
      leave `port` mode behavior unchanged
- [x] 1.3 Wire qualify (and any other callers) to pass `source_ip` into
      resolve; verify existing qualify destination tests still pass and
      a merging case holds when `source_ip` is `None`

## 2. Docs and examples

- [x] 2.1 Document both allocation modes and the Merging formula in
      `services/obake-ravenna/REFERENCE_GUIDE.md` and
      `services/obake-ravenna/README.md`; verify the guide names
      `allocation = "merging"`, `session_base`, and that
      `[multicast].address` is forbidden under merging
- [x] 2.2 Add a commented merging example beside the existing port
      sample in `services/obake-ravenna/examples/config.toml`; verify
      the file still loads under the default (port) uncommented path

## 3. Headless verification

- [x] 3.1 Run `cargo test` from `services/obake-ravenna` and verify all
      tests pass
