# Cheap runtime-host product-path evidence

Captured: 2026-05-12T22:50:10Z

Raw local log: `target/runtime-proof/cheap-runtime-host.log` (ignored, not committed).

## Commands and results

```bash
cargo nextest run -p aspen-jobs --test wasm_product_path_test --features plugins-wasm
# exit 0; 3 tests run: 3 passed, 0 skipped

cargo nextest run -p aspen-jobs --test oci_lowering_product_path_test --features plugins-wasm
# exit 0; 4 tests run: 4 passed, 0 skipped

cargo test -p aspen-jobs --test hyperlight_product_path_test --features plugins-vm -- product_path_marker_distinguishes_guardrail_from_execution_evidence --nocapture
# exit 0; 1 passed, 0 failed, 2 filtered out

cargo test -p aspen-jobs --test hermit_uhyve_product_path_test --features plugins-vm -- --nocapture
# exit 0; 4 passed, 0 failed, 1 ignored
```

## Boundary classification

- WASM product path: **product-path marker passed**. `JobManager`/`WorkerPool`/WASM worker orchestration reached the declared WASM execution marker, including an invalid-WASM negative path.
- OCI lowering product path: **product-path marker passed**. OCI-lowered WASM execution reached product orchestration, while model-only/raw-container paths did not satisfy the proof marker.
- Hyperlight non-ignored check: **guardrail/static distinction passed**. This check proves the marker does not confuse guardrail evidence with execution evidence; it is not a Hyperlight execution proof.
- Hermit/uHyve non-ignored checks: **guardrails/static product-path wrapping passed**. Fake-runner and negative-path receipt behavior passed; the real `hermit_uhyve_executes_declared_fixture_through_product_orchestration` test remained ignored until the gated Uhyve runner/image proof tier.

## Skipped / deferred from this tier

- Real Hyperlight ignored execution proof.
- Real Hermit/uHyve ignored execution proof using built `uhyve` and marker image.
- Nested-KVM VM snapshot runtime-host receipt.
- Full dogfood/self-hosting acceptance.

## Secret handling

No raw tickets, credentials, cluster cookies, private keys, or connection strings were retained in this committed summary.
