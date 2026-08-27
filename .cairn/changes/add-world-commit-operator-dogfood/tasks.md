## Phase 1: Workflow core

- [ ] [depends:add-world-commit-replay-capsules] Record baseline operator, simulation, receipt, reconciliation, and world-component tests. r[molten.world_operator.verification]
- [ ] [serial] Define typed world operation, workflow request, profile capability, expected observation, resource limit, operation graph, blocker, plan, receipt-link, and summary DTOs. r[molten.world_operator.plan] r[molten.world_operator.receipt]
- [ ] [depends:world-operator-dtos] Implement pure workflow planning, bound checks, dependency ordering, profile admission, first-blocker selection, and domain-separated BLAKE3 plan identity. r[molten.world_operator.plan]
- [ ] [parallel] Add canonical Preserves schemas for requests, plans, aggregate receipts, and summaries. r[molten.world_operator.receipt]

## Phase 2: CLI composition root

- [ ] [depends:world-operator-plan] Add `molten world` inspect, checkpoint, branch, run, diff, conflicts, replay, simulate, verify, promote, export, import, and GC-plan request parsing with no raw command text. r[molten.world_operator.commands]
- [ ] [depends:world-operator-commands] Compose existing application services and ports without moving domain logic into CLI dispatch. r[molten.world_operator.composition]
- [ ] [depends:world-operator-composition] Add preview-first mutation flow with exact plan identity and fresh mutable-observation rechecks. r[molten.world_operator.preview_apply]
- [ ] [depends:world-operator-composition] Emit aggregate workflow receipts that link component receipts and preserve their separate roles and non-claims. r[molten.world_operator.receipt]
- [ ] [parallel] Add bounded human and machine summaries with stable first-blocker diagnostics and redacted refs-only context. r[molten.world_operator.diagnostics]

## Phase 3: Dogfood and verification

- [ ] [depends:world-operator-preview-apply] Add the complete logical checkpoint-to-GC dogfood fixture with retained plans and receipts. r[molten.world_operator.dogfood]
- [ ] [depends:portable-chaoscontrol-snapshot-descriptor] Add one exact opaque restore and replay dogfood fixture without diff or merge claims. r[molten.world_operator.dogfood]
- [ ] [parallel] Add negative stale plan, implicit latest head, wrong generation, missing profile, denied authority, unresolved conflict, uncertain effect, incomplete capsule, unavailable witness, unsupported extent, raw command, secret disclosure, and GC-as-deletion-authority fixtures. r[molten.world_operator.verification]
- [ ] [serial] Document command contracts, preview/apply, component ownership, profile statuses, partial completion, receipts, and bounded dogfood claims. r[molten.world_operator.receipt]
- [ ] [depends:world-operator-verification] Run focused tests, the logical and opaque dogfood rails, Octet, Clippy with warnings denied, Cairn validation and gates, lifecycle checks, and relevant Nix checks. r[molten.world_operator.verification]
