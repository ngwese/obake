# Design

## Context

See proposal.md for motivation. `post-image.sh` builds `.raucb` files whenever
`RAUC_SIGNING_CERT` and `RAUC_SIGNING_KEY` are set. `--installer` always fills
those in (so the image keyring trusts later dev bundles), so both slots' bundles
are produced. Staging then `cp`s every top-level file in the Buildroot images
directory, including leftover `installer*.img` from earlier consoles. The copy
and the following volume `rm` print nothing.

## Goals / Non-Goals

**Goals:**

- Default `--installer` staged output to installer media plus the disk image,
  without `.raucb` files.
- Drop installer images that this build did not just produce.
- Print a line before cache staging and before cache cleanup.
- Keep an explicit override that still builds bundles during an installer build.

**Non-Goals:**

- Replacing the cache-volume copy with a bind mount.
- Changing `--update` defaults (bundles on, installer and disk off).
- Re-stamping git provenance (`-dirty` / short sha). That stamp is taken at
  startup and matched the tree when this build began.
- Changing how the installed image embeds the development keyring.

## Decisions

1. **Gate bundles on `MAKE_BUNDLE`, not on the presence of signing keys.**
   `--installer` exports `MAKE_BUNDLE=0` when unset. `--update` exports
   `MAKE_BUNDLE=1` when unset. An explicit `MAKE_BUNDLE` in the environment
   wins, matching `MAKE_DISK` / `MAKE_INSTALLER` on the update product.
   `post-image.sh` builds bundles only when `MAKE_BUNDLE=1` and signing
   material is set. Signing cert, key, and keyring stay exported for both
   products so the image still trusts same-host update bundles.
   Alternative: stop calling `ensure_dev_signing` for `--installer` — rejected;
   the keyring in the image is why signing is set.

2. **Remove stale installer images in the images directory before staging.**
   When this build writes `installer-<console>.img`, delete other
   `installer*.img` in that directory (including a bare `installer.img`).
   Staging can keep copying top-level files; leftovers are already gone.
   Also delete `*.raucb` from the staged directory when `MAKE_BUNDLE` is not
   1, so a bundle left in the cache from an earlier `--update` is not
   published. Same idea as today's `--update` strip of `installer*.img` and
   `disk-*.img`.

3. **Progress lines on the host, before each silent `docker run`.**
   Print `[build] staging artifacts...` before the copy and
   `[build] removing derived images from build cache...` before the volume
   `rm`. Do not redirect those messages away. Leave the copy mechanism as it
   is.

4. **Docs.** In `image/docs/user-guide.md`, describe `*.raucb` as an
   `--update` artifact, not "whenever signing is set".

## Risks / Trade-offs

- **[Risk] Harnesses that build with `--installer` and then look for a
  `.raucb` in that directory** → In-tree harnesses that need bundles use
  `--update` (or `MAKE_DISK=1` with `--update`). Confirm none expect a bundle
  from `--installer` alone.
- **[Risk] Operators who relied on `--installer` to also drop bundles** →
  They pass `MAKE_BUNDLE=1`, or run `--update` separately.
- **[Trade-off] Staging still copies the disk image and the new installer** →
  Intentional; this change does not remove that copy, it only stops extra
  bundles and stale installer files from being part of it.
