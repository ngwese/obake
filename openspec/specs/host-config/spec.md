# host-config Specification

## Purpose
Keeps the operating system root immutable and update-safe while giving users a
well-defined, persistent place to customize configuration, so that user changes
survive updates and updates never touch user data.

## Requirements

### Requirement: Read-only operating system root

The operating system root filesystem SHALL be mounted read-only during normal
operation, so that users cannot introduce changes to the root that would be
lost or would conflict with the next update.

#### Scenario: Root cannot be modified in place

- **WHEN** a process attempts to write to a path on the root filesystem that is
  not part of the writable contract
- **THEN** the write fails because the root is mounted read-only

### Requirement: Persistent configuration overlay

Changes made to system configuration under `/etc` SHALL persist across boots
and across slot switches by being stored in a writable overlay backed by the
persist partition.

#### Scenario: Configuration change survives reboot

- **WHEN** a user changes an allowed file under `/etc` and the host reboots
- **THEN** the change is still present

#### Scenario: Configuration survives slot switch

- **WHEN** the host switches from one slot to the other
- **THEN** the configuration stored in the overlay is unchanged

### Requirement: Volatile and persisted runtime state

The system SHALL provide a writable `/var` whose transient content is discarded
across boots and whose defined persisted subset survives across boots on the
persist partition.

#### Scenario: Transient state discarded

- **WHEN** the host reboots
- **THEN** transient `/var` content that is not part of the persisted subset is
  discarded

#### Scenario: Persisted subset retained

- **WHEN** the host reboots
- **THEN** the defined persisted subset under `/var` is retained

### Requirement: Persist partition

The system SHALL keep machine-specific state on a dedicated persist partition
that is shared across both slots and is not replaced by operating system
updates.

#### Scenario: Machine state independent of slot

- **WHEN** an operating system update activates the other slot
- **THEN** the machine-specific state on the persist partition is retained

### Requirement: Separate user partition untouched by updates

The system SHALL keep user data on a separate, writable user partition that is
mounted for user access and that SHALL NOT be modified by operating system
updates or by system reinstalls that preserve user data.

#### Scenario: Update leaves user data intact

- **WHEN** an operating system update is applied
- **THEN** no content on the user partition is modified

#### Scenario: Reinstall leaves user data intact

- **WHEN** the system is reinstalled while preserving user data
- **THEN** no content on the user partition is modified

### Requirement: Directory-contract customization

The system SHALL define a directory contract that names the locations that are
writable for host configuration and user data, including volatile `/media` as
the removable-media mount root. A user connecting over SSH as `obake` SHALL be
able to write under `/home`, and SHALL NOT be able to write `/etc` or the
read-only operating system root. System services MAY still write the `/etc`
overlay and other contract mounts as root.

#### Scenario: User customizes within the contract

- **WHEN** a user connected over SSH as `obake` modifies a file under
  `/home/obake`
- **THEN** the change persists on the user partition across boots and updates

#### Scenario: User cannot write /etc over SSH

- **WHEN** a user connected over SSH as `obake` attempts to create or modify a
  file under `/etc`
- **THEN** the modification fails

#### Scenario: User cannot modify outside the contract

- **WHEN** a user connected over SSH as `obake` attempts to modify the
  read-only root outside the contract
- **THEN** the modification fails

#### Scenario: Media mount root is listed as volatile

- **WHEN** an operator reads the directory contract
- **THEN** `/media` is named as the volatile removable-media mount root

### Requirement: Host user privileged access

The `obake` login user SHALL be able to perform system maintenance over SSH
without interactive authentication: managing systemd units and power state,
reading host diagnostics — the kernel log via `dmesg` and the full system
journal via `journalctl` — and applying or rolling back signed OS updates via
`obake-update`. Privileged actions that need root SHALL use a passwordless
sudo grant for all commands, including interactive root shells (`sudo -s`,
`sudo -i`, `sudo /bin/sh`). The grant SHALL NOT require unlocking the root
account password.

#### Scenario: Unit and power management over SSH

- **WHEN** the obake user runs `systemctl` unit or power operations over SSH
- **THEN** the action succeeds without a password prompt

#### Scenario: Kernel log readable

- **WHEN** the obake user runs `dmesg`
- **THEN** the kernel ring buffer is readable

#### Scenario: Full system journal readable

- **WHEN** the obake user reads the system journal with `journalctl`
- **THEN** entries from all services and the whole boot are readable

#### Scenario: Update apply without a password prompt

- **WHEN** the obake user runs `obake-update apply` over SSH with a valid
  bundle available on auto-mounted media or the configured HTTPS source
- **THEN** the update proceeds without a password prompt

#### Scenario: No general root via sudo

- **WHEN** the obake user runs `sudo -i` or `sudo /bin/sh`
- **THEN** sudo grants a root shell without a password prompt

#### Scenario: Arbitrary command via sudo

- **WHEN** the obake user runs `sudo -n <command>` for an arbitrary command
- **THEN** sudo runs the command as root without a password prompt

### Requirement: Locked root account

The image SHALL lock the root account so it has no usable password and local
`su` to root without credentials fails, while still allowing the system to
run root-owned services.

#### Scenario: su to root fails without credentials

- **WHEN** the `obake` user runs `su -` with an empty or incorrect password
- **THEN** authentication fails and the session remains unprivileged

#### Scenario: Root has no usable password

- **WHEN** an image is built
- **THEN** the root account password field is locked (not empty and not a
  valid hash an operator can type)

### Requirement: Removable media auto-mount for obake

When a removable storage partition appears, the system SHALL mount it
automatically under `/media/<name>` in the host mount namespace, where `<name>`
is the filesystem label when present, or a stable fallback derived from the
volume identity when unlabeled. `/media` SHALL be a writable mount root so
mountpoint directories can be created on the read-only OS image. The mount
SHALL be readable by `obake` without additional privilege and SHALL be
visible to processes that bind-mount `/media` (including Singularity).

#### Scenario: Labeled removable volume mounts under /media

- **WHEN** a removable partition with filesystem label `UPDATES` is attached
- **THEN** it is mounted at `/media/UPDATES` and the `obake` user can read
  files there without sudo

#### Scenario: Unlabeled removable volume still mounts

- **WHEN** a removable partition without a filesystem label is attached
- **THEN** it is mounted under `/media/` with a stable non-empty directory
  name derived from the volume identity

#### Scenario: Removal unmounts cleanly

- **WHEN** the removable media is detached
- **THEN** the corresponding `/media/<name>` mount is removed

#### Scenario: Media root accepts mountpoint directories

- **WHEN** the host has finished boot
- **THEN** `/media` is a writable mount root (not a directory on the
  read-only root alone) so a privileged process can create and remove a
  directory under `/media`

#### Scenario: Mount is visible for container bind

- **WHEN** a removable volume is auto-mounted under `/media/<name>`
- **THEN** that path is present in the host mount namespace so a Singularity
  bind of `/media` can expose the volume inside the container

### Requirement: ESP readable by obake

The EFI System Partition mounted at `/boot/efi` SHALL be readable and listable
by the `obake` login user without additional privilege. The `/boot` path prefix
SHALL be traversable by that user so ordinary diagnostics (for example `df`)
can resolve the mount without Permission denied.

#### Scenario: obake can list the ESP

- **WHEN** the `obake` user runs `ls /boot/efi` after boot
- **THEN** the command succeeds and lists ESP contents without sudo

#### Scenario: df can resolve the ESP mount

- **WHEN** the `obake` user runs `df` after boot
- **THEN** the command does not fail with Permission denied for `/boot/efi`

#### Scenario: Non-root cannot write the ESP

- **WHEN** the `obake` user attempts to create a file under `/boot/efi`
  without sudo
- **THEN** the write fails

### Requirement: Login accounting for last

The system SHALL provide login accounting files under `/var/log` so that the
`last` command can open `/var/log/wtmp` without error. After an SSH login as
`obake`, `last` SHALL report that session. Those accounting files SHALL be part
of the persisted `/var` subset so records survive reboot and slot switch.

#### Scenario: last opens wtmp

- **WHEN** the `obake` user runs `last` after the host has finished boot
- **THEN** the command does not fail with a missing `/var/log/wtmp` error

#### Scenario: SSH session appears in last

- **WHEN** the `obake` user has logged in over SSH and then runs `last`
- **THEN** the output includes an entry for the `obake` user

#### Scenario: Login records survive reboot

- **WHEN** an SSH login as `obake` has been recorded and the host reboots
- **THEN** `last` still shows that prior `obake` session

### Requirement: Optional network cabling

Wired network interfaces managed by the host SHALL NOT be required to reach an
online state before boot is considered successful. With no cable attached,
`systemd-networkd-wait-online` SHALL NOT leave the system in a failed or
degraded state solely because those interfaces lack carrier.

#### Scenario: Uncabled boot does not fail wait-online

- **WHEN** the host boots with no wired interface cabled
- **THEN** `systemd-networkd-wait-online.service` does not fail for lack of
  carrier on managed wired links

#### Scenario: DHCP still applies when a cable is present

- **WHEN** a managed wired interface gains carrier
- **THEN** that interface still obtains addressing via DHCP as configured

### Requirement: PAM for systemd and OpenSSH

The image SHALL include Linux-PAM and SHALL build systemd and OpenSSH with
PAM support. Interactive SSH authentication SHALL use PAM (`UsePAM yes`).

#### Scenario: PAM is present on the image

- **WHEN** a built image is inspected
- **THEN** Linux-PAM libraries and PAM configuration for `sshd` are present,
  and OpenSSH is configured to use PAM

#### Scenario: systemd is built with PAM

- **WHEN** a built image is inspected
- **THEN** systemd reports PAM support enabled

### Requirement: SSH login reaches the obake user bus

An interactive SSH login as `obake` SHALL provide the environment variables
needed to locate the lingering user runtime and session bus, so
`systemctl --user` works without manually exporting those variables. Those
variables SHALL come from PAM session setup (`pam_systemd`), not from a
login-shell `profile.d` workaround.

#### Scenario: systemctl --user works over SSH

- **WHEN** the `obake` user logs in over SSH after boot (with linger and the
  user manager already active)
- **THEN** `systemctl --user status` succeeds without the operator exporting
  `XDG_RUNTIME_DIR` or `DBUS_SESSION_BUS_ADDRESS`

#### Scenario: No profile.d user-bus workaround

- **WHEN** the image rootfs is inspected
- **THEN** it does not ship `/etc/profile.d/obake-user-bus.sh`

### Requirement: Volatile /tmp allows device nodes

The system SHALL mount `/tmp` as a tmpfs that is writable by the `obake`
login user, discarded across boots, mounted `nosuid`, and not mounted
`nodev` or `noexec`. The directory contract SHALL name `/tmp` as that
volatile mount.

#### Scenario: /tmp is a tmpfs without nodev

- **WHEN** a built image boots
- **THEN** `/tmp` is a tmpfs whose mount options include `nosuid` and do not
  include `nodev` or `noexec`, and `obake` can create a file there

#### Scenario: /tmp does not survive reboot

- **WHEN** `obake` creates a file under `/tmp` and the host reboots
- **THEN** that file is absent

#### Scenario: Contract lists /tmp as volatile

- **WHEN** an operator reads the directory contract
- **THEN** `/tmp` is named as a volatile tmpfs that is not persisted

### Requirement: obake manages its own login credentials

Authorized keys for the `obake` login user SHALL be read from
`~/.ssh/authorized_keys` on the user partition. The `obake` user SHALL be able
to add, replace, or remove those keys without elevated privileges, and SHALL be
able to change its own account password without elevated privileges when the
account already has a password. Setting a password on an account that has none
(an image built without `OBKA_PASSWORD`, whose password field is locked) MAY
require elevation. Credentials the user sets SHALL persist across boots, slot
switches, and operating system updates, and SHALL NOT be overwritten by an
update. The image SHALL NOT provision a permanently honored authorized key
outside the user's home, so replacing the user's key retires any seed.

#### Scenario: obake replaces its authorized key without elevated privileges

- **WHEN** the `obake` user writes its own `~/.ssh/authorized_keys` over an
  interactive or remote session
- **THEN** the write succeeds without sudo and subsequent SSH logins use the
  new key

#### Scenario: Replacing the key retires the seed

- **WHEN** the `obake` user replaces the contents of `~/.ssh/authorized_keys`
  after an install seeded a different key
- **THEN** the seeded key no longer grants SSH access

#### Scenario: obake changes an existing password without elevated privileges

- **WHEN** the `obake` account already has a password and the `obake` user
  changes it
- **THEN** the change succeeds without sudo and the new password works for
  console and SSH password login

#### Scenario: First password on a key-only host requires elevation

- **WHEN** the `obake` account has no password (an image built without
  `OBKA_PASSWORD`) and the `obake` user runs `passwd` without elevation
- **THEN** the change is refused, and setting the password requires the host's
  existing sudo grant or a build-time `OBKA_PASSWORD` seed

#### Scenario: User-set credentials survive an update

- **WHEN** the `obake` user has set its own key or password and the host
  applies an operating system update or switches slots
- **THEN** the user-set key and password remain in effect

### Requirement: obake renames the host without elevated privileges

The `obake` login user SHALL be able to set the host's static host name over
SSH without elevated privileges. The change SHALL take effect without a reboot
and SHALL persist across boots, slot switches, and operating system updates.
The image SHALL NOT rely on a hostname entry in `/etc/hosts`: the local name
SHALL resolve through the system resolver, so changing the host name does not
require editing `/etc/hosts`.

#### Scenario: obake sets the host name

- **WHEN** the `obake` user sets a new valid host name over SSH without sudo
- **THEN** the change succeeds and the host reports the new name

#### Scenario: Rename persists across a slot switch

- **WHEN** the `obake` user has renamed the host and the host then activates
  the other slot
- **THEN** the host still reports the new name

#### Scenario: No hostname entry in /etc/hosts

- **WHEN** an operator reads `/etc/hosts`
- **THEN** it does not map the host name, and the local host name still
  resolves

### Requirement: Network time synchronization

The image SHALL enable `systemd-timesyncd` so that, when network connectivity
and NTP servers are reachable, the system clock is synchronized from the
network. Boot success SHALL NOT require a successful time sync.

#### Scenario: timesyncd enabled on a fresh image

- **WHEN** a host boots an unmodified image
- **THEN** `systemd-timesyncd.service` is enabled

#### Scenario: timesyncd runs after boot

- **WHEN** a host reaches multi-user after boot
- **THEN** `systemd-timesyncd.service` is active

#### Scenario: Boot does not wait for NTP

- **WHEN** the host boots without reachable NTP servers
- **THEN** boot still completes successfully without waiting on time sync
