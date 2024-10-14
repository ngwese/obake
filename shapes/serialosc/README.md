# serialosc

## setup

- `sudo usermod -aG uucp dialout <your-user-name>`

## running

by default singularity does not bind mount /run/udev into the container. serialosc hotplug logic ultimately traverses symlinks under /sys to files under /run/udev, subsequent `open(...)` calls fail.

to map udev into the container, either:
- `SINGULARITY_BIND="/run/udev:/run/udev" ./serialosc.sif`
- `singularity run --bind /run/udev:/run/udev serialosc.sif`
