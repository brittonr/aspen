# OCI artifact lowering model/admission/docs evidence

## Implementation evidence

- `crates/aspen-runtime-core/src/lib.rs`
  - Demotes `RuntimeHostKind::OciContainer` to a dev/unsafe raw-container marker.
  - Adds `OciLoweringTarget`, `OciLoweringPlan`, and `OciLoweringReceipt` portable DTOs.
  - Adds pure admission helpers that reject raw production containers, mutable/non-matching OCI identities, missing isolated targets, invalid derived artifacts, undeclared handles, and secret-bearing lowering diagnostics/receipts.
  - Adds positive OCI-to-microVM lowering tests and negative raw-container/unscoped/secret tests.
- `docs/runtime-applications.md`
  - Describes OCI as content-addressed artifact ingestion/lowering rather than a Podman/Docker-style production host boundary.
  - Documents microVM default lowering and specialized Hyperlight/WASM/unikernel targets.
- `tests/runtime_host_loading_docs_test.rs`
  - Anchors `OciLoweringPlan` and the dev/unsafe-only `RuntimeHostKind::OciContainer` language so ordinary containers are not reintroduced as the default production boundary.

## Focused validation

```text
rustfmt crates/aspen-runtime-core/src/lib.rs
CARGO_TARGET_DIR=target/agent cargo test -p aspen-runtime-core oci --all-targets

running 3 tests
oci_admission_rejects_raw_containers_and_unscoped_lowering ... ok
oci_images_lower_to_isolated_microvm_boundary ... ok
oci_lowering_receipts_reject_secret_bearing_material ... ok
```

No registry credentials, raw environment secrets, mutable tags as durable identity, ambient host paths, tokens, or private material are included in the evidence.
