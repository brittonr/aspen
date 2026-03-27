## ADDED Requirements

### Requirement: Bootable serial VM image with two clusters

The Nix package SHALL produce a qcow2 disk image containing two independent aspen-node instances (alice and bob) configured for federation, bootable via `vm_boot` with serial console access.

#### Scenario: VM boots with both services

- **WHEN** the image is booted with `vm_boot` and auto-login completes
- **THEN** `systemctl status aspen-node-alice` and `systemctl status aspen-node-bob` both show active

### Requirement: Helper scripts in VM

The VM SHALL include scripts at `/etc/dogfood-federation/` for each pipeline step (init, push, federate, sync, build, verify).

#### Scenario: Full pipeline via serial

- **WHEN** user runs `/etc/dogfood-federation/full-test.sh` via `vm_serial`
- **THEN** the script executes the full federated dogfood pipeline and prints a success/failure summary

### Requirement: Flake integration

The VM image SHALL be buildable via `nix build .#dogfood-serial-federation-vm` and produce `result/disk.qcow2`.

#### Scenario: Build and boot

- **WHEN** `nix build .#dogfood-serial-federation-vm` completes
- **THEN** `result/disk.qcow2` exists and boots successfully under QEMU with UEFI firmware
