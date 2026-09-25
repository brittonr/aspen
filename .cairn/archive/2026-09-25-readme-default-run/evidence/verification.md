# Verification: Make README `cargo run --` commands resolve the molten binary

Base: `9e5f6db73` (`octet-burndown-safety-ports` on `integration/stack-20260925`).

## Spot runs (`spot-runs.txt`), run with no `--bin`

1. `cargo run -- test run examples/two-actor.preserves --report-out target/molten-reports/two-actor.report.preserves`:
   exit 0, report `blake3:a65df018…` written.
2. `cargo run -- test report validate target/molten-reports/two-actor.report.preserves`: exit 0.
3. `cargo run -- test octet gate --artifacts <strict artifacts> --profile strict-ci --receipt-out …`: the gate ran and
   wrote its receipt, then denied as expected. These artifacts are `warning-only`, and they predate the Cargo.toml
   change, so the config hash is also stale.

Before this change all three failed with "`cargo run` could not determine which binary to run".

## Build plans

- `Cargo.lock`, `build-plan.json`, and `release-policy-build-plan.json` are byte-unchanged (`git diff --quiet`).
- `nix eval .#packages.x86_64-linux.default.drvPath` succeeds. This runs unit2nix's `cargoLockHash` staleness check.
- `nix build .#checks.x86_64-linux.git-source-hash-binding` passes.

## Gates

- fmt: exit 0. clippy `-D warnings`: exit 0. `cargo test --workspace --no-fail-fast`: 2091 passed, 0 failed.
- Pinned Octet: workspace 3209 → 3209 and lib 1363 → 1363. The per-lint counts are identical.

## Review and lifecycle

The no-spec plan review approves the plan with no findings, the three gates pass, and strict validate reports no
issues.
