# Spec Delta

## ADDED Requirements

### Requirement: Local display console for hardware boards

A board with no serial console SHALL include a kernel build that drives a text
console from the firmware-provided EFI framebuffer, so an attached monitor and
USB keyboard can be used without a GPU driver that the realtime configuration
excludes.

#### Scenario: Firmware framebuffer text console

- **WHEN** the qdl image is built
- **THEN** the kernel binds a framebuffer console to the EFI framebuffer and
  provides a virtual terminal for local output

#### Scenario: Console does not require an RT-incompatible GPU driver

- **WHEN** the qdl kernel is built with the realtime fragment
- **THEN** the local console works without i915, which the realtime
  configuration excludes
