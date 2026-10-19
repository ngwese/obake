# Tasks

## 1. Factory config and unit rlimits

- [x] 1.1 Add `image/host/pipewire/pipewire.conf.d/10-obake-rt.conf` with
  `module.rt.args` setting `rtkit.enabled = false` and
  `rtportal.enabled = false`, and verify the file is present under
  `image/host/pipewire/pipewire.conf.d/`
- [x] 1.2 Add `image/host/wireplumber/wireplumber.conf.d/10-obake-rt.conf`
  with the same `module.rt.args` block, and verify it sits beside
  `10-obake-alsa.conf`
- [x] 1.3 Add `image/host/systemd/wireplumber.service.d/10-obake-rt.conf`
  matching `pipewire.service.d/10-obake-rt.conf`
  (`LimitRTPRIO=95`, `LimitRTTIME=infinity`, `LimitMEMLOCK=infinity`), and
  verify the three keys match the PipeWire drop-in
- [x] 1.4 Update `image/os/buildroot/package/obake-host-config/obake-host-config.mk`
  to install `systemd/wireplumber.service.d/*.conf` into
  `/usr/lib/systemd/user/wireplumber.service.d/`, and verify the install
  recipe mirrors the existing `pipewire.service.d` block

## 2. Verification and docs

- [x] 2.1 Extend `check_pipewire_runtime` in
  `image/verification/guest/obake-verify.sh` to require
  `wireplumber.service` `LimitRTPRIO=95`, and verify the function asserts
  both units
- [x] 2.2 Update `image/host/SETUP.md` and the rtkit paragraph in
  `image/os/README.md` to mention both unit drop-ins and the factory
  `module.rt.args` files, and verify those paths are named in the docs
- [x] 2.3 Rebuild or refresh a host image that includes `obake-host-config`
  and run the headless QEMU guest functional check that exercises
  `check_pipewire_runtime`, and verify it passes with both units reporting
  `LimitRTPRIO=95`
