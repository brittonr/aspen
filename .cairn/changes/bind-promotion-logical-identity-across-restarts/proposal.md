# Bind promotion logical identity across restarts

## Why

`LocalWorldPromotionStore` already writes the successor head, promotion record, and reservation set in one Redb transaction, and a commit error becomes `OutcomeUnknown` rather than an invented result. Dispatch design also preserves logical reservation identity across retry attempts. But no tracked contract separates the four identity roles that the reliable-submission model requires: the logical operation, the canonical request bound to that operation, the individual attempt, and the generation that authorizes the acting owner.

Without that contract, two failure modes stay untested: a retry can silently become a new logical operation after a restart, and two intentionally distinct operations with identical payload hashes can be merged by deduplication. The TigerBeetle reliable-submission review (2026-09) identified this as the highest-value promotion-path gap. The finding is static source review; no live promotion fault was executed.

## What Changes

- Add an explicit four-way identity contract to the promotion path: logical operation identity, canonical request identity, attempt identity, and generation/fencing identity, each carried in bounded promotion records. r[molten.promotion_identity.roles]
- Reject a retry that reuses a logical operation identity with incompatible canonical request parameters. r[molten.promotion_identity.request_binding]
- Keep two distinct logical operations with identical payloads separate; payload-hash equality alone never merges operations. r[molten.promotion_identity.no_payload_merge]
- Preserve the original logical operation identity across `OutcomeUnknown`, process restart, and retry; a retry never mints a new logical identity. r[molten.promotion_identity.restart_binding]
- Add the key failure regression: commit the reservation, execute or possibly execute the effect, lose the response, restart with the same storage, and retry using the original logical identity. r[molten.promotion_identity.validation]

## Impact

- **Files**: `molten-core` promotion store types and transitions, dispatch retry planning, Redb adapter records, promotion docs.
- **Testing**: four-role round-trip, incompatible-parameter rejection, duplicate-payload non-merge, restart replay with the original identity, `OutcomeUnknown` uncertainty retention, Redb reopen and malformed-record negatives.
- **Non-goals**: no exactly-once claim for external effects, no new authoritative reservation ledger, no change to existing authority or completion-owner checks.

## Dependencies

- `bind-delivery-duplicate-results-to-original-claims` establishes operation-bound duplicate responses on the delivery path; this change applies the same principle to promotion and stays implementable without it.
- `add-world-head-rollback-witnessing` separately owns authority recovery; this change does not.
