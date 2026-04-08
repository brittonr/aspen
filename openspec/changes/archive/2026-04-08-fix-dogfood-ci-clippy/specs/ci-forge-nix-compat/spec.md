## ADDED Requirements

### Requirement: Forge checkout produces valid flake directory

The CI executor SHALL produce a working directory from Forge-checked-out source that `nix build` can evaluate as a flake. The directory MUST contain `flake.nix`, `flake.lock`, and be a git repository (so nix detects it as a git flake).

#### Scenario: Clippy check on Forge checkout

- **WHEN** the CI pipeline runs a nix job with `flake_url = "."` and `attribute = "checks.x86_64-linux.clippy"` on source checked out from Forge
- **THEN** the nix build SHALL evaluate the flake and execute the clippy check successfully (exit code 0)

#### Scenario: flake.lock preserved through Forge push

- **WHEN** source is pushed to Forge via `git-remote-aspen`
- **THEN** the `flake.lock` file SHALL be present in the Forge repository and available in CI checkouts

#### Scenario: CI working directory is a git repo

- **WHEN** the CI executor prepares a working directory for a nix flake job
- **THEN** the directory SHALL be initialized as a git repository with all source files committed, so that `nix build .#<attr>` resolves the flake correctly

### Requirement: CI nix jobs work without network access

The CI executor's nix build for Forge source SHALL succeed without network access to resolve flake inputs. All inputs MUST be resolvable from the local nix store or binary cache.

#### Scenario: Vendored deps available in CI sandbox

- **WHEN** the nix build runs inside a sandbox for a Forge checkout
- **THEN** all flake inputs referenced in `flake.lock` SHALL resolve from the nix store (pre-fetched or cached), without requiring network fetches during evaluation
