## Context

`ObservedPromotionCurrent` (in `src/world_promotion/material/mod.rs`) supplies `WorldPromotionTransactionFacts` from the stored head and from signed authority and closure files. `validate_promotion_transaction` runs inside the Redb write transaction, so the head recheck is atomic with publication. The closure check compares only reference strings. `plan_retry_for_reservation` and `abandon_uncertain_for_attempt` rebuilt the plan with `plan_world_promotion(request)`, which requires the request document's `authority_admitted` and `intent_closure_complete` to be true, and the CLI passed no operator authority to either. Deny compared a signed grant's `admitted` and `generation` inline in the CLI.

## Decisions

### Decision: An intent reference is the content address of canonical intent material

**Choice:** A canonical `<world-effect-intent-material "molten.world-effect-intent-material.v1" <semantic-ref …> <handler-ref …> <adapter-ref …> <release-class …>>` record. Its BLAKE3 content reference is the intent reference. `LocalIntentMaterial` stores it at `world-promotion/intent-material/<hex>.preserves` in the storage namespace `Scope` with a 64 KiB bounded read. `read` derives the reference from the bytes and returns `None` for absent, non-canonical, or readdressed material; `publish` is idempotent and refuses to overwrite differing bytes.

**Rationale:** Content addressing makes "the material named by the signed closure" exact without a second signature. Encoding semantic, handler, adapter, and release class lets one comparison detect a request that declares another adapter than its material. The store follows the existing content-addressed receipt store and the node-state capability rules; no ambient path is reconstructed.

### Decision: Material availability is a transaction fact decided by the pure validator

**Choice:** `WorldPromotionTransactionFacts` gains `intent_material_available`. The adapter computes it during `observe_transaction`, immediately before the write transaction, with the pure `intent_material_matches(plan, material)`. `validate_promotion_transaction` adds `IntentMaterialUnavailable`.

**Rationale:** Keeps I/O in the shell and the decision in `molten-core`. A distinct issue keeps missing material separate from an incomplete signed closure in diagnostics.

### Decision: One operator-authority decision for deny, retry planning, and abandon

**Choice:** `read_operator_authority` returns a verified `WorldOperatorAuthority`. The pure `validate_operator_authority(authority, reservation, plan)` always requires `admitted`, `generation == reservation.generation`, matching branch, authority and policy, and a reservation belonging to the committed original plan. Deny, retry planning and abandon all require the original request document and check before writing. Deny can change only committed or blocked reservations, or repeat an existing denial; it cannot replace an in-flight or uncertain effect outcome.

**Rationale:** A grant for another branch at the same generation must not deny this reservation. Mandatory immutable-plan binding gives every operator action the same scope, while preserving the separate acknowledgement requirements for effect uncertainty.

### Decision: Rebuild the original plan from durable identity, not caller booleans

**Choice:** `rebuild_committed_plan` sets the planning inputs to the facts the committed transaction validated (`admitted`, observed generation equal to expected, closure complete), plans, and accepts the plan only if `read_promotion_identity(operation)` returns that exact `WorldPromotionIdentity` (or a legacy record with the same plan reference).

**Rationale:** The canonical request identity covers branch, heads, generation, policy, authority, and every intent, and it is persisted only by a validated commit. A request that differs in any of these, or names an uncommitted operation, cannot rebuild a different plan, so no replacement logical identity is reachable. Authority comes from the operator grant, not the document.

## Risks / Trade-offs

- Existing request documents and signed closures must use material-derived intent references; arbitrary references now deny with `IntentMaterialUnavailable`. No production promotion has run against the signed-adapter profile.
- Operator grants and closures remain signed by one trusted key supplied on the command line; this change does not add key rotation or an external authority source.
