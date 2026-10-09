# Design

## Context

See proposal.md for motivation. The shared base
(`image/os/buildroot/configs/obake_base.config`) already pulls OpenSSL
(via Singularity) and libcurl (via `obake-runtime` / `obake-update`), but
does not enable Buildroot's `ca-certificates` package. Guest checks in
`image/verification/guest/obake-verify.sh` cover Singularity presence and
bind-mount files, not the TLS trust store. RAUC signing uses a separate
keyring at `/etc/rauc/ca.cert.pem` installed by `post-build.sh`; that path
must stay independent of the system TLS store.

## Goals / Non-Goals

**Goals:**

- Enable the shared Mozilla/system CA bundle for every board from
  `obake_base`.
- Provide a documented, deterministic build-time hook for extra root CA
  PEMs that land in the same store OpenSSL/curl/Singularity consult.
- Fail headless guest verification when the store is missing or empty.
- Keep offline/reproducible builds by vendoring any new package sources.

**Non-Goals:**

- Runtime / persist-partition CA management (operators drop certs into a
  live host after install).
- Changing RAUC signing, `tls_cacert` on `obake-update`, or shape images.
- Networked guest tests that pull `library://` or hit Sylabs (QEMU guests
  may lack outbound HTTPS); presence of the store is the gate.
- Disabling TLS verification as a workaround.

## Decisions

1. **Enable `BR2_PACKAGE_CA_CERTIFICATES` in `obake_base.config`** — one
   shared switch for every board; matches how bash/openssh/etc. are shared.
   Alternative: board-only enablement (rejects the "every image verifies
   HTTPS" contract).

2. **Extra CAs as PEM/CRT files under a fixed board path** — operators place
   `*.crt` (PEM) files in
   `image/os/buildroot/board/obake/extra-ca-certificates/` (empty by
   default; tracked with a README). `post-build.sh` copies them into
   `usr/share/ca-certificates/local/` and regenerates
   `/etc/ssl/certs/ca-certificates.crt` the same way Buildroot's
   `ca-certificates` finalize hook does (that hook runs before post-build,
   and `update-ca-certificates` is removed from the target). Alternative:
   env var pointing at an arbitrary host directory (flexible, but less
   discoverable); optional later as an override that still feeds the same
   install step.

3. **Keep RAUC keyring separate** — do not add RAUC's release CA to the
   system store or vice versa. Update-client `tls_cacert` for private
   update servers remains the existing optional path in `update.conf`.

4. **Guest check: non-empty system bundle** — add
   `check_ca_certificates` in `obake-verify.sh` asserting
   `/etc/ssl/certs/ca-certificates.crt` exists and has size > 0. Do not
   require outbound HTTPS from QEMU. Alternative: openssl verify against a
   fixture cert (more coverage, more fixture maintenance); defer unless
   presence checks prove insufficient.

5. **Vendor `ca-certificates` into `vendor/dl/`** — same offline pin
   contract as other base packages. Build once with network if needed,
   then commit the fetched source via git-lfs.

## Risks / Trade-offs

- **[Risk] Extra CA dir accidentally ships secrets or huge PEMs** →
  Mitigation: document PEM-only root certs; no private keys; keep the
  default tree empty in git.
- **[Risk] Finalize hook order leaves extras out of the bundle** →
  Mitigation: regenerate the bundle in `post-build.sh` after installing
  extras (mirror `CA_CERTIFICATES_GEN_BUNDLE`); verify the subject appears
  in the built rootfs bundle.
- **[Risk] Guest has no network so library:// is untested** → Mitigation:
  accept store-presence check; optional manual smoke on hardware remains
  `singularity pull/run library://lolcow`.
- **[Trade-off] Build-time extras only** → Personal CAs require rebuilding
  (or OTA of a custom-built image); matches "base build" request and keeps
  the read-only root contract clean.

## Migration Plan

- Rebuild any board image; the default public CA set appears automatically.
- Operators who need private roots add PEMs under
  `board/obake/extra-ca-certificates/` and rebuild (or bake them into a
  CI/release image).
- Existing hosts pick up the store on the next OTA slot activation.
- Rollback: revert the base config / post-build / guest check commits; no
  persist migration.
