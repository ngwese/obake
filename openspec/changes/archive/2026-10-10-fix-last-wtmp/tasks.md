# Tasks

## 1. Runtime login accounting files

- [x] 1.1 Add
  `image/os/buildroot/board/obake/rootfs-overlay/etc/tmpfiles.d/obake-wtmp.conf`
  creating `/var/log/wtmp` (0664 root utmp), `/var/log/btmp` (0660 root utmp),
  and `/var/log/lastlog` (0664 root utmp); verify the overlay file is present
  and matches those modes/owners
- [x] 1.2 Change
  `image/os/buildroot/board/obake/rootfs-overlay/etc/obake/persisted-var` from
  `log/journal` to `log`; verify the list no longer names `log/journal` alone
- [x] 1.3 Update `image/docs/host-config.md` persisted-`/var` section to list
  `log` (journal + wtmp family) and mention that `last` uses `/var/log/wtmp`;
  verify the doc matches `persisted-var`

## 2. OpenSSH records logins

- [x] 2.1 In `image/os/buildroot/external.mk`, filter out OpenSSH
  `--disable-lastlog`, `--disable-utmp`, `--disable-utmpx`, `--disable-wtmp`,
  and `--disable-wtmpx` from `OPENSSH_CONF_OPTS`; verify the snippet is present
  next to the existing RAUC override
- [x] 2.2 Rebuild the image (e.g. qdl or verification board) and confirm the
  target `sshd` was built without `DISABLE_WTMP` /
  `DISABLE_WTMPX` in its `config.h` (or equivalent build log evidence)

## 3. Headless verification

- [x] 3.1 Extend `image/verification/qemu/ssh-test.sh` so that after SSH login
  as `obake`, the harness fails unless `last` succeeds and its output includes
  `obake`; verify by reading the new assertion in the script
- [x] 3.2 Run `image/verification/qemu/ssh-test.sh` headlessly and verify it
  reports PASS with the new `last` check
