# Hyperlight gated product proof evidence

Captured: 2026-05-13T03:12:34Z

Raw logs are kept under ignored `target/runtime-proof/hyperlight-gated-proof.log`; this committed summary omits full build logs and contains no cluster tickets or credentials.

## Command

```bash
mkdir -p target/runtime-proof && set -o pipefail; \
  cargo test -p aspen-jobs --test hyperlight_product_path_test --features plugins-vm \
    hyperlight_job_executes_declared_fixture_through_product_orchestration -- --ignored --nocapture \
  2>&1 | tee target/runtime-proof/hyperlight-gated-proof.log
```

Result: exit 0.

## Proof markers and boundary

Test output:

```text
test hyperlight_job_executes_declared_fixture_through_product_orchestration ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.12s
```

The gated test asserts the completed Hyperlight `vm_execute` job:

- passed through Aspen `JobManager` plus `WorkerPool` orchestration (`attempts > 0`);
- returned `JobStatus::Completed` with `JobResult::Success`;
- used the declared Hyperlight ABI `aspen:runtime-host/hyperlight-v1` and entrypoint `execute`;
- set the product-visible receipt marker to `ASPEN_HYPERLIGHT_RUNTIME_HOST_EXECUTED`;
- preserves the separate negative guardrail marker `ASPEN_HYPERLIGHT_RUNTIME_HOST_PRODUCT_PATH_GUARD` for non-proof paths.

Classification: reached the final Hyperlight runtime-host product receipt boundary on this host: the ignored gated test executed the declared Hyperlight fixture through Aspen product orchestration and asserted the product-visible receipt marker.
