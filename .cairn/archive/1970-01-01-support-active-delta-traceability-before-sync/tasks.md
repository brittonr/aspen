## Phase 1: Implementation

- [x] [serial] Separate accepted coverage/debt from pending identity resolution in the existing standalone guard, retaining exact baseline enforcement. r[molten.project.inherited_tracey_debt.active_delta_scope] r[molten.project.inherited_tracey_debt.growth_denial]
- [x] [serial] Restrict pending discovery to native active delta specifications and ADDED/MODIFIED requirements; exclude removed identities and non-delta documents. r[molten.project.inherited_tracey_debt.active_delta_admission]
- [x] [serial] Expose pending reference counts and explicit identity-only non-claims without changing sync/archive policy. r[molten.project.inherited_tracey_debt.active_delta_non_claims]

## Phase 2: Verification

- [x] [serial] Exercise positive pending resolution, accepted/pending overlap, accepted debt growth and stale debt denial, unknown identity denial, removed identity denial, and non-delta/archive exclusion through behavioral tests and the actual guard CLI. r[molten.project.inherited_tracey_debt.active_delta_validation]
- [x] [serial] Run focused formatting, the inherited Tracey Nix check, strict Cairn validation, and proposal/design/tasks gates; preserve actual outcomes and any independent blocker without weakening policy. r[molten.project.inherited_tracey_debt.active_delta_validation]

## Phase 3: Review and lifecycle

- [x] [serial] Review the scope/coverage separation and negative evidence; update existing traceability documentation and record reproducible commands, maintenance ownership, and non-claims. r[molten.project.inherited_tracey_debt.active_delta_non_claims] r[molten.project.inherited_tracey_debt.active_delta_validation]
- [x] [serial] Confirm every implementation, validation, and review task is supported by passing evidence; review the non-mutating native sync plan and archive preconditions; record the preflight identities and unchanged-policy execution sequence. r[molten.project.inherited_tracey_debt.active_delta_validation]

## Mandatory post-checklist lifecycle completion

The acceptance checklist is a preflight, not completion authority. After every checkbox is supported and complete, regenerate the native sync preview, execute sync only with the reviewed unchanged native input, policy, profile, action, delta, and merge-preflight identities, and retain its successful mutation receipt and exact expected-output hash match. Then regenerate the native archive preview, execute archive only with its reviewed unchanged native inputs and actions, and retain its successful mutation receipt outside the moved package. Cairn includes execution mode and mutation observations in its envelope `plan_hash`; preserve preview and execution hashes separately rather than asserting that those different envelopes hash identically. A blocked gate or changed planned input requires resolution and a new preview, not a waiver. The change remains incomplete until both native operations succeed and the accepted specification, archived package, and receipts agree. No checklist item or dry-run receipt substitutes for actual sync/archive execution.
