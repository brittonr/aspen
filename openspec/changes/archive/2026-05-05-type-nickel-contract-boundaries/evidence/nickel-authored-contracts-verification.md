# Nickel-authored Contract Verification

Completed: 2026-05-05T23:08:44Z

## Scope

This slice completes the remaining Nickel-authored families in `type-nickel-contract-boundaries`:

- CI pipeline/deploy config contracts.
- Node/cluster/profile contracts and feature bundle policy.
- Test harness and patchbay/network manifest metadata.
- Snix/build executor policy and trust/bootstrap policy.
- Crate-extraction readiness policy contracts.
- Operator diagnostics evidence family placeholder boundary.

## Implementation Summary

- Deepened `crates/aspen-ci/src/config/schema/ci_schema.ncl` with typed timeout, retry, dependency, cache, artifact, deploy statefulness, and validation-only metadata contracts.
- Deepened `crates/aspen-nickel/src/schema/node_config.ncl` with typed identity, bootstrap topology, feature bundle, storage, transport/discovery, metrics/OTLP, and trust/quorum references.
- Deepened `test-harness/schema.ncl` with capabilities, isolation assumptions, timeout classes, expected artifacts, fault dimensions, and convergence assertions.
- Deepened `docs/crate-extraction/policy.ncl` with readiness/class/publication/evidence contract coverage.
- Added new typed policy contracts:
  - `schemas/feature-bundles.ncl`
  - `schemas/snix-build-executor-policy.ncl`
  - `schemas/trust-bootstrap-policy.ncl`
  - `schemas/operator-diagnostics-evidence.ncl`
- Added `scripts/check-typed-nickel-contract-fixtures.py` to typecheck touched contracts and export positive/negative fixtures.
- Updated `schemas/typed-nickel-contract-registry.ncl` and `docs/typed-nickel-contract-registry.md` with validation commands for the completed contract families.

## Verification Commands

Passed:

```bash
python3 scripts/check-typed-nickel-contract-fixtures.py
python3 scripts/check-typed-nickel-contract-registry.py
python3 scripts/generate-typed-nickel-contracts.py --check
python -m py_compile scripts/check-typed-nickel-contract-fixtures.py scripts/check-typed-nickel-contract-registry.py scripts/generate-typed-nickel-contracts.py
nix run nixpkgs#nickel -- typecheck schemas/typed-nickel-contract-registry.ncl
nix run nixpkgs#nickel -- typecheck crates/aspen-ci/src/config/schema/ci_schema.ncl
nix run nixpkgs#nickel -- typecheck crates/aspen-nickel/src/schema/node_config.ncl
nix run nixpkgs#nickel -- typecheck test-harness/schema.ncl
nix run nixpkgs#nickel -- typecheck docs/crate-extraction/policy.ncl
nix run nixpkgs#nickel -- typecheck schemas/feature-bundles.ncl
nix run nixpkgs#nickel -- typecheck schemas/snix-build-executor-policy.ncl
nix run nixpkgs#nickel -- typecheck schemas/trust-bootstrap-policy.ncl
nix run nixpkgs#nickel -- typecheck schemas/operator-diagnostics-evidence.ncl
nix run nixpkgs#nickel -- typecheck schemas/dogfood-run-receipt.ncl
nix run nixpkgs#nickel -- typecheck schemas/ci-run-receipt.ncl
nix run nixpkgs#nickel -- typecheck schemas/deploy-protocol.ncl
cargo test -p aspen-ci test_deploy_protocol_schema_snapshot
cargo nextest run -p aspen-ci test_deploy_protocol_schema_snapshot
cargo test -p aspen-client-api ci_receipt_schema_and_status_labels_are_documented --features ci
scripts/test-harness.sh check
scripts/check-crate-extraction-readiness.rs --policy docs/crate-extraction/policy.ncl --inventory docs/crate-extraction.md --manifest-dir docs/crate-extraction --candidate-family trust-crypto-secrets --output-json openspec/changes/type-nickel-contract-boundaries/evidence/crate-extraction-readiness.json --output-markdown openspec/changes/type-nickel-contract-boundaries/evidence/crate-extraction-readiness.md
```

Notable fixture output:

```text
typed Nickel fixture checks OK: 12 typechecks, 7 positive exports, 2 negative exports
typed Nickel registry OK: 12 families, 12 Crunch classifications, 6 non-candidates
typed Nickel generated contracts fresh: 2 files
```

The fixture checker includes negative exports for invalid CI timeout ordering and invalid trust quorum threshold, proving contract failures are detected.
