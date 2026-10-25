# Tasks

## 1. Pure helpers (obake-ravenna)

- [x] 1.1 Add DESCRIBE path selection (TXT `path=` else
  `/by-name/<session name>`) and verify unit tests cover present path,
  absent path, and `path=/by-name/...` TXT forms
- [x] 1.2 Add device/session grouping by resolved IPv4+port (orphans in
  an unassigned section) and verify unit tests for matched, orphan, and
  multi-session device cases
- [x] 1.3 Add human report formatting for devices and session SDP
  summaries (and failure reasons) and verify unit tests match the
  intended layout for success and `describe-failed` / `sdp-invalid` rows

## 2. Avahi browse/resolve and RTSP client (obake-ravenna)

- [x] 2.1 Implement one-shot Avahi ServiceBrowser for both RAVENNA
  subtypes on a given ifindex with settle `--timeout` (default 3s),
  ResolveService to IPv4+port+TXT, and verify a headless unit or
  recording test covers request construction without requiring a live
  bus when possible
- [x] 2.2 Implement a short-timeout RTSP DESCRIBE client that returns
  the SDP body or an error, parse with `parse_sdp`, and verify unit
  tests for a fixture 200 response and a timeout/error path

## 3. CLI wiring and docs (obake-ravenna)

- [x] 3.1 Add `obake-ravenna browse` (`--ptp4l-config`, `--timeout`) that
  resolves the PTP ifindex, runs browse/resolve/DESCRIBE, prints the
  report, exits non-zero on missing interface / Avahi failure, and
  verify `cargo test` in `services/obake-ravenna` passes headlessly
- [x] 3.2 Document `browse` in `services/obake-ravenna/README.md` and
  `services/obake-ravenna/REFERENCE_GUIDE.md` (device vs session, path
  default, timeout) and verify the docs name the command and summary
  fields
