## MODIFIED Requirements

### Requirement: Nix evaluation supports configurable target system

The snix eval path SHALL accept a target system parameter instead of assuming `x86_64-linux`. The system string flows from the build payload through evaluation to derivation analysis.

#### Scenario: End-to-end system propagation

- **WHEN** a CI job specifies `system: "aarch64-linux"` in its build payload
- **THEN** the flake-compat expression, derivation resolution, and build request all use `aarch64-linux`

### Requirement: Flake lock input resolution handles submodules

The flake lock resolver SHALL pass the `submodules` flag from locked git inputs to the fetch layer, enabling builds of flakes with git submodule dependencies.

#### Scenario: Flake with submodule dependency builds correctly

- **WHEN** a flake.lock references a git input with `"submodules": true`
- **THEN** the fetched source includes submodule contents and the build succeeds
