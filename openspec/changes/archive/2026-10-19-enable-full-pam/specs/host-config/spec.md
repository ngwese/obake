# Spec Delta

## ADDED Requirements

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

## MODIFIED Requirements

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
