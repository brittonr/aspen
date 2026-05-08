# Verification

Change: `review-testing-harness-public-api-followup`

## Implementation summary

- Added reusable structured inventory freshness diagnostics in `aspen_testing::suite_inventory`:
  - `InventoryCheckReport`
  - `InventoryCheckDiagnostic`
  - `InventoryCheckSeverity`
  - `check_inventory_current`
  - `render_inventory_check_diagnostics`
- Added `aspen-test-harness check --json` for machine-readable freshness diagnostics while keeping existing human `check` output.
- Added `scripts/check-aspen-testing-public-api-boundary.py` and `scripts/test-harness.sh public-api-boundary` to guard the default `aspen-testing --no-default-features` dependency graph.
- Included `public-api-boundary` in `scripts/test-harness.sh quick-confidence` so the dependency boundary stays part of the bounded local preflight.
- Documented the reusable default API and explicit adapter feature boundary in `docs/testing-harness-public-api.md`, linked it from `README.md`, and updated quick-confidence docs.

## Commands run

```bash
nix run .#rustfmt
cargo run -p aspen-testing --bin aspen-test-harness -- check --json
scripts/test-harness.sh public-api-boundary
cargo test -p aspen-testing suite_inventory::tests::structured_inventory_check_reports_current_and_stale_states -- --nocapture
cargo test --test testing_harness_public_api_docs -- --nocapture
cargo test --test quick_confidence_rail_docs -- --nocapture
```

All commands above passed. The public API boundary report returned:

```json
{
  "crate": "aspen-testing",
  "leaked_packages": [],
  "missing_explicit_features": [],
  "mode": "no-default-features",
  "status": "passed"
}
```

## Archive validation

Before archive, run:

```bash
scripts/test-harness.sh quick-confidence --summary target/quick-confidence/public-api-followup.json
openspec validate review-testing-harness-public-api-followup --strict --json
git diff --check
```

After archive, `openspec archive` initially left an extra blank line at EOF in `openspec/specs/testing-harness-extraction/spec.md`; trimmed it to exactly one trailing newline, then ran:

```bash
git diff --check
openspec validate --all --strict --json
```

Post-archive validation passed with `219/219` valid OpenSpec items.
