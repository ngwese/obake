# Proposal

## Why

The `obake` login account is created with no password, so access is SSH-key
only. Boards with a local text console (and operators who want a simple
password fallback) need a way to set that password when the image is built,
without changing the default key-only posture.

## What Changes

- Accept an optional build-time password for the `obake` account (via an
  environment variable, alongside existing knobs such as
  `OBKA_SSH_AUTHORIZED_KEY`).
- When the knob is unset or empty, keep today's behavior: password login
  disabled; SSH key access unchanged.
- When the knob is set, the built image's `obake` account accepts that
  password for local console login, and SSH password authentication as
  `obake` is enabled and usable with that password.
- Document the knob and its security implications (hash lives in the image
  rootfs; OTA replaces it with whatever the new image carries).
- Do **not** seed the password onto the persist partition (unlike SSH
  authorized keys); it is an image property, not machine-specific state.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `os-image-build`: optional build-time password for the `obake` login
  account; default remains no password.

## Impact

- `image/os/buildroot/board/obake/`: apply the password during rootfs assembly
  (users table and/or post-fakeroot) and, when set, ensure sshd allows
  password authentication for `obake`, without changing the default
  `users.txt` when the knob is unset.
- `image/build/build.sh`, `image/build/build-env/run.sh`, and
  `image/verification/verify-env/run.sh`: forward the new build knob into the
  containerized build/verify environments.
- `image/docs/user-guide.md`, `image/os/README.md`: document the knob and
  default key-only behavior.
- OTA/RAUC and the installer persist contract are unchanged; a password set at
  build time travels with the slot image and is replaced on update.
- Cross-host shape portability is unaffected.
