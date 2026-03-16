## ADDED Requirements

### Requirement: Snix bridge import works with in-memory backends

The `aspen-snix-bridge` in standalone mode (no cluster ticket) SHALL successfully handle `snix-store import` for files and directories through its gRPC interface.

#### Scenario: Import a single file

- **WHEN** `snix-store import /tmp/test.txt` is run against the bridge's gRPC socket
- **THEN** the output SHALL contain a valid `/nix/store/` path
- **AND** the bridge process SHALL remain running

#### Scenario: Import a directory tree

- **WHEN** `snix-store import /tmp/test-dir/` is run against the bridge's gRPC socket
- **THEN** the output SHALL contain a valid `/nix/store/` path

### Requirement: MicroVM reads imported data via virtiofs

After importing data through the bridge, a cloud-hypervisor microVM using snix-store virtiofs SHALL be able to read the imported store paths from `/nix/store`.

#### Scenario: MicroVM sees imported file

- **WHEN** a file is imported via the bridge
- **AND** a microVM is booted with virtiofs pointing at the same bridge socket
- **THEN** the microVM's `/nix/store` SHALL contain the imported path

### Requirement: snix-bridge-virtiofs VM test passes

The `snix-bridge-virtiofs-test` NixOS VM test SHALL pass end-to-end.

#### Scenario: Full test passes

- **WHEN** `nix build .#checks.x86_64-linux.snix-bridge-virtiofs-test` is run
- **THEN** all subtests SHALL pass including bridge startup, file import, microVM boot, and store path visibility
