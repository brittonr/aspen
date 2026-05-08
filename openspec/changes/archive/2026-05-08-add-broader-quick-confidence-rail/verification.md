# Verification: Broader quick confidence rail

## Initial bounded command set

The rail composes existing local checks instead of inventing new semantic proof classes:

- `scripts/test-harness.sh check`
- `scripts/test-harness.sh runtime-host-acceptance-bundle`
- `cargo test --test operator_receipts_docs -- --nocapture`
- `cargo test --test runtime_host_readiness_docs -- --nocapture`
- `openspec validate --all --strict --json`
- `git diff --check`

The rail explicitly skips and names full dogfood, KVM/NixOS VM runtime-host proofs, Uhyve/Hermit runtime-host proofs, Hyperlight runtime-host proofs, network/ignored nextest profiles, and full `nix flake check`.

## Focused verification

```bash
scripts/test-harness.sh quick-confidence --dry-run --json --summary target/quick-confidence/dry-run-test.json
cargo test --test quick_confidence_rail_docs -- --nocapture
scripts/test-harness.sh quick-confidence --summary target/quick-confidence/summary.json
openspec validate add-broader-quick-confidence-rail --strict --json
git diff --check
```

Post-archive validation:

```bash
git diff --check
openspec validate --all --strict --json
```

Observed quick rail result:

- status: `passed`
- included checks: 6/6 passed
- structured summary written to `target/quick-confidence/summary.json`
- non-proof boundary included in text and JSON output
