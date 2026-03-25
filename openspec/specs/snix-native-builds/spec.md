## MODIFIED Requirements

### Requirement: Subprocess fallback

The executor SHALL retain the `nix build` subprocess execution path as a runtime fallback. The `nix-cli-fallback` feature flag is removed — the subprocess path is always compiled in when `snix-build` is enabled, and activates only when the native build service fails or is unavailable at runtime.

#### Scenario: Fallback to nix CLI

- **WHEN** `snix-build` feature is enabled
- **AND** the native `BuildService` fails to initialize or a build fails
- **THEN** the executor SHALL fall back to spawning `nix build` as a subprocess
- **AND** the executor SHALL log a warning indicating fallback was triggered
