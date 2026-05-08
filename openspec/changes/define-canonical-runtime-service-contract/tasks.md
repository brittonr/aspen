## Phase 1: Contract Foundation

- [x] [serial] Create the OpenSpec baseline for the canonical runtime-service contract.
- [ ] [serial] Inventory existing job, plugin, deploy, route, and receipt types that should become contract adapters.

## Phase 2: Model and Adapter Slices

- [ ] [depends:inventory] Add or extend portable model types for service/backend/receipt correlation without adding scheduler side effects.
- [ ] [depends:model] Implement the first narrow adapter slice for one existing backend or built-in service.
- [ ] [depends:adapter] Add route/health boundary tests that distinguish declared, pending, active, withdrawn, failed, and stopped states.

## Phase 3: Verification

- [ ] [depends:tests] Run focused model/adapter tests, relevant receipt/docs checks, OpenSpec validation, and whitespace checks.
