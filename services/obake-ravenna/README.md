<!--
SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
SPDX-License-Identifier: MIT
-->

# obake-ravenna

RAVENNA session daemon for the obake host. It browses and publishes RAVENNA
RTSP services, serves and retrieves SDP, and loads PipeWire RTP modules on
the existing `obake` graph while `ptp4l` reports a lock.

## Configuration

Live file (seeded once on `qdl`):

`/home/obake/.config/obake-ravenna/config.toml`

```toml
[[source]]
name = "obake-out-1"
channels = 8
format = "L24"          # L16 or L24
frame_size = 48         # samples per packet
destination = "239.69.1.10:5004"
ttl = 15                # optional, default 15

[[sink]]
name = "obake-in-1"
device = "Studio-IO"    # DNS-SD instance name
stream = "PGM-1"
```

Sample rate is not configured here. It is the live PipeWire `clock.rate`.
Accepted rates are the AES67 / RAVENNA set: 44100, 48000, 88200, 96000,
176400, 192000, 352800, and 384000 Hz. The network interface is the
non-`global` section in `/home/obake/.config/linuxptp/ptp4l.cfg`.

Frame size must yield one of 1/8, 1/4, 1/3, 2/3, 1, 4/3, or 4 ms at
48 kHz-family rates (for example at 48000 Hz: 6, 12, 16, 32, 48, 64, or
192 samples; at 96000 Hz: 12, 24, 32, 64, 96, 128, or 384). On the
44.1 kHz family those exact ptimes are not an integer number of samples,
so the same absolute sample counts as at 48000 Hz are accepted.

## Hold-down reasons

`obake-ravenna status` reports why a stream is inactive:

| Reason | Meaning |
|--------|---------|
| `unlocked` | `ptp4l` is inactive or not in SLAVE |
| `bad-sample-rate` | graph rate is not an AES67 / RAVENNA media clock |
| `bad-frame-size` | frame size is not an allowed packet time at the graph rate |
| `bad-destination` | destination is not scoped multicast on an even port |
| `duplicate-destination` | another local source uses the same group and port |
| `interface-missing` | `ptp4l` interface is absent or still a placeholder |
| `device-absent` | sink device/stream not discovered |
| `unicast-sdp` | remote SDP connection address is not multicast |
| `rate-mismatch` | remote SDP rate differs from the graph rate |
| `domain-mismatch` | remote SDP PTP domain differs from local `ptp4l` |

## Commands

```
obake-ravenna run
obake-ravenna status
```

Reload the user service to re-read the config without restarting PipeWire.
