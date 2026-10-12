# Spec Delta

## ADDED Requirements

### Requirement: Build-time SSH authorized key seed

The build SHALL accept an optional public SSH key for the `obake` login user
via the `OBKA_SSH_AUTHORIZED_KEY` environment variable, whose value names a
public key file. The build SHALL read that file regardless of where it resides
on the build host, including outside the build tree, and SHALL behave the same
on macOS and Linux build hosts. When set to a readable file, the key SHALL be
embedded in the installer medium as a seed that the installer provisions for
the `obake` user, and the image SHALL NOT provision a permanently honored key
outside that user's home. When unset or empty, no key seed SHALL be embedded.
When set to a path that is not a readable file, the build SHALL fail with an
error naming `OBKA_SSH_AUTHORIZED_KEY`.

#### Scenario: Key outside the build tree is honored

- **WHEN** an image is built with `OBKA_SSH_AUTHORIZED_KEY` naming a readable
  public key file that resides outside the repository
- **THEN** the build succeeds and the key is embedded in the installer medium
  as the `obake` seed

#### Scenario: Build host platform does not change the result

- **WHEN** the same image is built from a macOS host and from a Linux host with
  the same key input
- **THEN** both builds succeed and embed the same key seed

#### Scenario: Unreadable key path fails the build

- **WHEN** an image is built with `OBKA_SSH_AUTHORIZED_KEY` naming a path that
  is not a readable file
- **THEN** the build fails with an error naming `OBKA_SSH_AUTHORIZED_KEY`

#### Scenario: Unset key embeds no seed

- **WHEN** an image is built without `OBKA_SSH_AUTHORIZED_KEY` set
- **THEN** the installer medium carries no authorized key seed

## MODIFIED Requirements

### Requirement: Optional build-time obake password

The build SHALL accept an optional cleartext password for the `obake` login
account via the `OBKA_PASSWORD` environment variable. When unset or empty, the
account SHALL have no password and SSH password authentication as `obake`
SHALL fail. When set, the built image SHALL accept that password for local
console login and SHALL allow SSH password authentication as `obake` with that
password. The build-time password hash SHALL be written to the image rootfs
and SHALL NOT be seeded onto the persist partition; activating a different
slot's image replaces the seed password unless the `obake` user has already
changed it, in which case the user-set hash is retained by the persistent
`/etc` overlay.

#### Scenario: Default has no password

- **WHEN** an image is built without `OBKA_PASSWORD` set
- **THEN** the `obake` account has no usable password and SSH password
  authentication as `obake` fails

#### Scenario: Password set enables console and SSH password login

- **WHEN** an image is built with a non-empty `OBKA_PASSWORD`
- **THEN** the `obake` account accepts that password for local console login
  and an SSH client can authenticate as `obake` using that password

#### Scenario: Password is not persist state

- **WHEN** an image built with `OBKA_PASSWORD` is installed and the `obake`
  user has not changed the password
- **THEN** the build-time password hash is not seeded onto the persist
  partition and is replaced when a different slot image is activated

#### Scenario: Unchanged seed password is replaced on update

- **WHEN** a host that has not changed the seed password activates a different
  slot whose image was built with a different `OBKA_PASSWORD`
- **THEN** the `obake` account accepts the new image's password, not the old one

#### Scenario: User-changed password survives update

- **WHEN** the `obake` user changes its own password and the host later
  activates a different slot
- **THEN** the user-set password remains in effect and is not replaced by the
  slot image's seed
