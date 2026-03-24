## ADDED Requirements

### Requirement: Eval uses build payload system when provided

The `evaluate_flake_derivation` function SHALL use the `system` field from `NixBuildPayload` when it is set, instead of hardcoding `x86_64-linux`.

#### Scenario: Payload specifies aarch64-linux

- **WHEN** `NixBuildPayload.system` is `Some("aarch64-linux")`
- **THEN** the evaluation uses `aarch64-linux` as the system parameter in the flake-compat expression

#### Scenario: Payload system is None

- **WHEN** `NixBuildPayload.system` is `None`
- **THEN** the evaluation detects the host system and uses it (e.g., `x86_64-linux` on an x86_64 Linux host)

### Requirement: Host system detection maps correctly to Nix system strings

The system detection SHALL map OS and architecture to valid Nix system strings: `x86_64-linux`, `aarch64-linux`, `x86_64-darwin`, `aarch64-darwin`.

#### Scenario: Linux x86_64 host

- **WHEN** the host runs Linux on x86_64
- **THEN** the detected system is `x86_64-linux`

#### Scenario: macOS aarch64 host

- **WHEN** the host runs macOS on aarch64
- **THEN** the detected system is `aarch64-darwin`

### Requirement: Flake lock parser reads submodules field

The flake lock git input parser SHALL read the `submodules` boolean from locked input nodes when present, and default to `false` when absent.

#### Scenario: Locked input includes submodules true

- **WHEN** a flake.lock git node contains `"submodules": true`
- **THEN** `fetch_and_clone_git` is called with `submodules = true`

#### Scenario: Locked input omits submodules field

- **WHEN** a flake.lock git node does not contain a `submodules` field
- **THEN** `fetch_and_clone_git` is called with `submodules = false`
