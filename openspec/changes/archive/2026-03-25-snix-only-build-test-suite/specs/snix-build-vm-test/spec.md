## ADDED Requirements

### Requirement: Native build succeeds without nix CLI in PATH

A NixOS VM integration test SHALL boot a node with `snix-build` enabled, confirm `nix` is not in `$PATH`, and execute a build job that completes via the native pipeline.

#### Scenario: nix binary absent from PATH

- **WHEN** the VM boots with `aspen-node` and `bwrap` installed but `nix` NOT in `$PATH`
- **THEN** `which nix` exits non-zero and `which nix-store` exits non-zero

#### Scenario: Native build of trivial derivation succeeds

- **WHEN** a pre-instantiated `.drv` file and its input closure exist in the VM's `/nix/store`, and a `ci_nix_build` job is submitted pointing at a flake directory
- **THEN** the job completes successfully, the output store path exists, and build logs contain "native build completed"

#### Scenario: Build does not spawn nix subprocess

- **WHEN** the build job completes
- **THEN** `journalctl -u aspen-node` does NOT contain "nix-store" or "nix eval" or "nix build" subprocess invocation errors (ENOENT for nix binary)

### Requirement: Fallback subprocess paths produce clear errors when nix absent

When `nix` is not in PATH and a code path attempts to shell out, the error SHALL be caught and reported — not silently swallowed.

#### Scenario: compute_input_closure falls back to direct inputs

- **WHEN** `nix-store` is not available and `compute_input_closure` is called
- **THEN** it returns the direct input names (fallback path) and logs a warning — does NOT panic or hang

#### Scenario: ensure_flake_lock fails gracefully

- **WHEN** `nix` is not available and `ensure_flake_lock` is called
- **THEN** it returns `Err` with a descriptive message — does NOT panic

### Requirement: VM test registered in flake checks

The new VM test SHALL be registered as `checks.x86_64-linux.snix-pure-build-test` in the flake and runnable via `nix build .#checks.x86_64-linux.snix-pure-build-test --impure`.

#### Scenario: Flake check exists

- **WHEN** `nix flake show --json` is run
- **THEN** the output contains `checks.x86_64-linux.snix-pure-build-test`
