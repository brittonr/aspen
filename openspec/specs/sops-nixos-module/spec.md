## ADDED Requirements

### Requirement: NixOS module overlay

A NixOS module SHALL override `sops.package` with the `aspen-sops-install-secrets` package. The module SHALL be importable alongside the upstream sops-nix module.

#### Scenario: Module sets sops package

- **WHEN** the module is imported in a NixOS configuration
- **THEN** `config.sops.package` is set to the `aspen-sops-install-secrets` binary

#### Scenario: Module composes with upstream sops-nix

- **WHEN** the module is imported after sops-nix's module
- **THEN** all sops-nix options (secrets, age, templates) work as before, using the Rust binary for decryption

### Requirement: Cluster ticket configuration

The module SHALL expose `sops.aspenTransit.clusterTicket` option. When set, the `ASPEN_CLUSTER_TICKET` environment variable is added to the sops-install-secrets execution environment.

#### Scenario: Cluster ticket passed to binary

- **WHEN** `sops.aspenTransit.clusterTicket` is set to a ticket string
- **THEN** the sops-install-secrets binary receives `ASPEN_CLUSTER_TICKET` in its environment

#### Scenario: No cluster ticket uses age-only decryption

- **WHEN** `sops.aspenTransit.clusterTicket` is not set
- **THEN** the binary runs without `ASPEN_CLUSTER_TICKET` and decrypts using age keys only

### Requirement: Nix package for aspen-sops-install-secrets

The flake SHALL export an `aspen-sops-install-secrets` package built with unit2nix (or crane). The package SHALL include only the `aspen-sops-install-secrets` binary with no runtime dependencies beyond libc.

#### Scenario: Package builds from flake

- **WHEN** `nix build .#aspen-sops-install-secrets` is run
- **THEN** a package containing `bin/sops-install-secrets` is produced

#### Scenario: Binary name matches Go convention

- **WHEN** the package is installed
- **THEN** the binary is named `sops-install-secrets` (not `aspen-sops-install-secrets`) for drop-in compatibility with sops-nix's expected binary name

### Requirement: Integration test

A NixOS VM integration test SHALL verify the full lifecycle: encrypt a SOPS file with `aspen-sops`, boot a VM with the sops-nix module using `aspen-sops-install-secrets`, and verify secrets are decrypted at the correct paths with correct permissions.

#### Scenario: Age-only decrypt at boot

- **WHEN** a SOPS file is encrypted with an age recipient matching the VM's key
- **THEN** the secret is decrypted during activation and available at `/run/secrets/<name>`

#### Scenario: Transit + age decrypt at boot

- **WHEN** a SOPS file is encrypted with both Transit and age, and the cluster is reachable
- **THEN** Transit is used for decryption (age is fallback)
