# Research: ci-dogfood-full-loop offline fixture failure

## Current failing rail

- Command: `nix build .#checks.x86_64-linux.ci-dogfood-full-loop-test --no-link -L`
- Derivation log: `/nix/store/3ip63zr7b9hbi3901c7nba8qbh1a02nx-vm-test-run-ci-dogfood-full-loop.drv`
- Failure point: VM test subtest `3-stage pipeline succeeds`.

## Observed pipeline state

The VM reached the Aspen CI pipeline:

- `format-check` shell job completed successfully.
- `syntax-check` Nix job was submitted as `ci_nix_build` for `.#checks.x86_64-linux.cargo-check`.
- The workflow marked `syntax-check` failed and moved the job to the dead-letter queue.
- Later `build-and-test` and `unit-tests` stayed pending because the `check` stage failed.

## Root error excerpt

The Nix build job failed while updating the sample flake lock/input graph:

```text
error: unable to download 'https://channels.nixos.org/flake-registry.json': Could not resolve hostname (6) Could not resolve host: channels.nixos.org

… while updating the lock file of flake 'git+file:///tmp/ci-checkout-...'
… while updating the flake input 'nixpkgs'
```

## Interpretation

This is a fixture determinism failure, not accepted evidence that Aspen CI stage ordering is broken. The sample flake in `nix/tests/ci-dogfood-full-loop.nix` uses:

```nix
inputs.nixpkgs.url = "nixpkgs";
```

and the generated repo has no `flake.lock`, so guest `nix build` attempts public registry resolution. The full-loop rail should instead use local/store-resident inputs or an input-free fixture while preserving three-stage CI orchestration proof.
