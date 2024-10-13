# siren

an environment for musical control, synths, effects, expression.

## details

### jackd

contains configuration files for jackd intended for used with
parameterized systemd unit file distributed with and installed by
jack2 repo. the config files should be copied to `/etc/jackd/` of the
target system. 

provided configs:

- `usbpre`: sounddevices usbpre 2
- `mixpre`: sounddevices mixpre ii series (tested with mixpre 6 mk ii)

### udev

udev rule to allow, relax permissions, and/or tweak configuration of
usb devices when pluged in. rules should be copied to
`/etc/udev/rules.d/`

provided rules:

- `50-mixpre6.rules`: explicitly switches usb driver from uac1 to uac2
  mode for increased channel and sampling rate options
- `51-usbpre2.rules`: starts jackd on connection (does not work)
- `58-push2.rules`: sets device permissions to `0666`
