# Tasks

## 1. System CA package in shared base

- [x] 1.1 Enable `BR2_PACKAGE_CA_CERTIFICATES=y` in
      `image/os/buildroot/configs/obake_base.config` and verify
      `check-boards.sh` still reports a clean base/board split with no
      unexpected base-symbol overlap
- [x] 1.2 Vendor the `ca-certificates` source into
      `image/os/buildroot/vendor/dl/` (git-lfs) and verify a board build
      resolves it from the offline cache

## 2. Build-time extra root CAs

- [x] 2.1 Add `image/os/buildroot/board/obake/extra-ca-certificates/`
      (empty default with README documenting PEM `*.crt` only, no private
      keys) and verify the directory is present in the tree
- [x] 2.2 Extend `image/os/buildroot/board/obake/post-build.sh` to copy any
      `*.crt` from that directory into the target local CA path and
      regenerate `/etc/ssl/certs/ca-certificates.crt`, and verify a rebuild
      with a test root CA produces a larger or updated bundle that
      includes that CA subject
- [x] 2.3 Document the extra-CA path in `image/os/README.md` (or
      `image/docs/user-guide.md`) and verify the documented rebuild steps
      match the implemented directory and file naming

## 3. Headless verification

- [x] 3.1 Add `check_ca_certificates` to
      `image/verification/guest/obake-verify.sh` asserting
      `/etc/ssl/certs/ca-certificates.crt` exists and is non-empty, and
      verify the check is invoked in the guest run list
- [x] 3.2 Run a headless image build (e.g.
      `image/os/buildroot/build.sh generic-x86_64` or the verification
      path that exercises guest checks) and verify the built rootfs
      contains a non-empty system CA bundle and guest verification reports
      the new CA check as pass
