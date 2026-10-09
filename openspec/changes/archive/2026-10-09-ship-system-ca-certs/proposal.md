# Proposal

## Why

HTTPS clients on the installed host (Singularity library pulls, curl, and any
other TLS consumer) fail with `x509: certificate signed by unknown authority`
because the image does not ship a system CA trust store. Operators also need a
supported way to bake private registry / personal root CAs into the base image
so self-signed endpoints verify without disabling TLS.

## What Changes

- Ship the standard system CA certificate bundle in every board image via the
  shared Buildroot base (so public HTTPS endpoints verify out of the box).
- Accept additional root CA certificates at image build time and merge them
  into that trust store, so personal or air-gapped registries can be trusted
  without runtime package installs or `tls_insecure` workarounds.
- Cover the trust store in headless guest verification so a missing CA bundle
  fails CI-style checks before release.
- Document how to add extra CA PEMs for a personal build.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `os-image-build`: require a system CA trust store in the base userland, plus
  a build-time path to merge additional root CAs into that store.
- `image-verification`: assert the installed image exposes a usable system CA
  trust store during guest functional checks.

## Impact

- `image/os/buildroot/configs/obake_base.config`: enable the Buildroot
  `ca-certificates` package (shared by every board).
- `image/os/buildroot/board/obake/`: build-time directory and/or post-build
  hook to install extra root CA PEMs into the system trust store.
- `image/os/buildroot/vendor/dl/`: pin/vend the `ca-certificates` (and any
  newly required) source for offline deterministic builds.
- `image/verification/guest/obake-verify.sh`: guest check that the CA bundle
  is present and usable for TLS verification.
- Docs under `image/docs/` or `image/os/README.md`: how to supply extra CAs.
- OTA: hosts receive the trust store (and any baked-in extras) with the next
  slot image; no persist-partition migration. RAUC's own signing keyring
  (`/etc/rauc/ca.cert.pem`) remains separate from the system TLS store.
- Shapes unchanged; Singularity library and HTTPS registry pulls become
  verifiable once the host image includes the store.
