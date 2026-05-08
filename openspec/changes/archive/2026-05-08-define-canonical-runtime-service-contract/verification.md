# Verification

Change: `define-canonical-runtime-service-contract`

## Implementation summary

- Added canonical portable runtime service contract types in `aspen-runtime-core`:
  - `RuntimeServiceContract`
  - `RuntimeServiceContractState`
  - `RuntimeExecutionBackendKind`
  - `RuntimeRouteState`
  - `RuntimeRouteObservation`
  - `RuntimeServiceReceiptCorrelation`
- Added pure helpers:
  - `runtime_backend_kind`
  - `canonical_runtime_service_contract`
  - `runtime_route_state_for_health`
  - `runtime_route_observations`
  - `runtime_receipt_correlation`
- Added `AdmissionError::EmptyHostLoadingReference` so a contract cannot be emitted without an explicit host-loading reference.
- Documented the contract in `docs/runtime-service-contract.md` and linked it from the README.
- Added `tests/runtime_service_contract_docs.rs` so docs preserve the validated-contract-is-not-activation boundary and receipt-correlation wording.

## Commands run

```bash
nix run .#rustfmt
cargo test -p aspen-runtime-core canonical -- --nocapture
cargo test -p aspen-runtime-core route_observations -- --nocapture
cargo test -p aspen-runtime-core receipt_correlation -- --nocapture
```

Before archive, also run:

```bash
cargo test --test runtime_service_contract_docs -- --nocapture
openspec validate define-canonical-runtime-service-contract --strict --json
git diff --check
```

After archive, `openspec archive` initially left an extra blank line at EOF in `openspec/specs/runtime-service-core/spec.md`; trimmed it to exactly one trailing newline, then ran:

```bash
git diff --check
openspec validate --all --strict --json
```
