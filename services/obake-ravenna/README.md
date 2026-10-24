<!--
SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
SPDX-License-Identifier: MIT
-->

# obake-ravenna

RAVENNA session daemon for the obake host. It browses and publishes RAVENNA
RTSP services, serves and retrieves SDP, and creates PipeWire RTP objects on
the existing `obake` graph while `ptp4l` reports a lock.

The compiler is pinned to Rust 1.99.1 via `rust-toolchain.toml`. The qdl
Buildroot package overrides `host-rust-bin` to the same version.

## Build requirements

Host builds of this crate link `libpipewire` through the [`pipewire`](https://crates.io/crates/pipewire)
crate. Install the development packages and a Clang shared library for
`bindgen` (used by `pipewire-sys`):

```
# Debian / Ubuntu
sudo apt install pkg-config libpipewire-0.3-dev libspa-0.2-dev libclang-dev
```

`pkg-config` must resolve `libpipewire-0.3` and `libspa-0.2`. The qdl
Buildroot package already depends on `obake-pipewire` and uses the staging
sysroot; these packages are only needed when building on a developer machine.

## Configuration

Live file (seeded once on `qdl`):

`/home/obake/.config/obake-ravenna/config.toml`

See `examples/config.toml` for a typical setup. Schema sketch:

```toml
[graph]
rate = 48000
node_group = "obake.ravenna"

# RTP frames per packet (not the PipeWire quantum).
[packet]
frames = 48

[multicast]
address = "239.1.60.50"
base_port = 5004          # even; source i uses base_port + 2*i
ttl = 15                  # optional, default 15

[[source]]
name = "obake-out-1"
channels = 8
format = "L24"            # L16 or L24
# payload = 104           # optional; default 96 + channels
# address = "239.1.60.51" # optional pin
# port = 5010             # optional even pin

[[sink]]
name = "obake-in-1"
device = "Anubis"
stream = "Anubis_Combo 1/2"
```

An empty `[[source]]` / `[[sink]]` list transmits nothing and joins no
multicast groups (the image seed starts empty).

Accepted rates are the AES67 / RAVENNA set: 44100, 48000, 88200, 96000,
176400, 192000, 352800, and 384000 Hz. The network interface is the
non-`global` section in `/home/obake/.config/linuxptp/ptp4l.cfg`.

`[packet].frames` must yield one of 1/8, 1/4, 1/3, 2/3, 1, 4/3, or 4 ms at
48 kHz-family rates (for example at 48000 Hz: 6, 12, 16, 32, 48, 64, or
192 samples). On the 44.1 kHz family the same absolute sample counts as at
48000 Hz are accepted. The PipeWire quantum is independent of this value.

## Hold-down reasons

`obake-ravenna status` reports why a stream is inactive:

| Reason | Meaning |
|--------|---------|
| `unlocked` | `ptp4l` is inactive or not in SLAVE |
| `no-grandmaster` | no grandmaster identity from `ptp4l` |
| `bad-sample-rate` | configured rate is not an AES67 / RAVENNA media clock |
| `bad-frame-size` | frames per packet is not an allowed packet time at the rate |
| `bad-destination` | destination is not scoped multicast on an even port |
| `duplicate-destination` | another local source uses the same group and port |
| `payload-too-large` | audio payload exceeds 1440 bytes |
| `bad-payload-type` | payload type outside 96..=127 |
| `interface-missing` | `ptp4l` interface is absent or still a placeholder |
| `device-absent` | sink device/stream not discovered |
| `unicast-sdp` | remote SDP connection address is not multicast |
| `rate-mismatch` | remote SDP rate differs from the configured rate |
| `domain-mismatch` | remote SDP PTP domain differs from local `ptp4l` |

## Commands

```
obake-ravenna run
obake-ravenna status
```

Logging uses `env_logger`. Default filter is `info` (also set on the user
unit). For full traces including PTP TLVs, discovery, and each tick:

```
RUST_LOG=debug obake-ravenna run
```

Reload the user service to re-read the config without restarting PipeWire.
