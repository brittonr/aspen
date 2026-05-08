# Tasks

## 1. Contract
- [x] Define a canonical service contract model for runtime service identity, host-loading reference, backend kind, artifact identity, capability/resource policies, receipt policy, declared routes, and contract state.
- [x] Add a validation helper that admits `RuntimeServiceSpec` and refuses empty host-loading references before producing a canonical contract.
- [x] Represent backend kinds separately from route/health state so native built-ins, WASM, Hyperlight, microVMs, Hermit/Uhyve, external processes, and deploy actions share one contract vocabulary.

## 2. Lifecycle/receipts
- [x] Add route observation state helpers that distinguish declared/pending/active/withdrawn/failed routes.
- [x] Ensure a route is only active when its instance is both running and healthy.
- [x] Add receipt correlation data linking service generation, optional instance ID, backend execution ID, artifacts, routes, and receipt ID.

## 3. Documentation/tests
- [x] Document the runtime service contract and the non-activation boundary for validated contracts.
- [x] Add positive and negative tests for contract validation, route activation gating, and receipt correlation.
- [x] Run targeted Rust/doc tests and OpenSpec validation before archive.
