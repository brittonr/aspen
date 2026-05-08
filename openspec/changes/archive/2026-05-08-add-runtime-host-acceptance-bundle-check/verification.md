# Verification

## Commands

- `scripts/check-runtime-host-acceptance-bundle.py`
- `scripts/test-harness.sh runtime-host-acceptance-bundle`
- `cargo test --test runtime_host_readiness_docs -- --nocapture`

## Evidence

The static acceptance-bundle check validates all five promoted runtime-host rows against operator documentation, suite manifests, generated inventory, source proof markers, and non-proof wording. The harness wrapper first runs the generated-inventory freshness check, then invokes the bundle check.

The docs guardrail test passed with 6 runtime-host readiness assertions.
