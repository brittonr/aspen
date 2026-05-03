## Context

Aspen dogfood already orchestrates the core self-hosting path, but its durable state is operational (`dogfood-state.json`) rather than evidentiary. If a run fails after pushing, building, or deploying, the operator mostly has log output. Crunch's release evidence design shows a better shape: small typed records, schema constants, bounded collections, canonical JSON, artifact linkage, and validation before trusting a proof bundle.

## Goals / Non-Goals

**Goals:**

- Add a typed dogfood run receipt model that can be serialized and validated independently of live cluster execution.
- Keep the first slice small: model, canonical JSON, validation, and tests.
- Reserve live receipt writing and final receipt path printing for follow-up tasks in the same change.
- Preserve current dogfood CLI behavior until receipt emission is wired in.

**Non-Goals:**

- No new Aspen RPC or wire protocol in the first slice.
- No cryptographic signing or release attestation yet.
- No full dogfood run during the first model slice unless the environment makes it cheap.
- No replacement of OpenSpec evidence; receipts complement OpenSpec evidence.

## Decisions

### 1. Receipt model lives inside `aspen-dogfood`

**Choice:** Add `receipt.rs` to the dogfood binary crate first.

**Rationale:** The first consumer is the dogfood orchestrator. A separate reusable crate would add decomposition overhead before the shape proves useful.

**Alternative:** Create `aspen-dogfood-core`. Rejected for the first slice because there is no external consumer yet.

**Implementation:** `main.rs` declares `mod receipt;`; the module owns constants, enums, structs, validation, and JSON helpers.

### 2. Crunch-inspired schema constants and validation

**Choice:** Use a schema string constant, bounded stage/artifact counts, explicit validation, and deterministic JSON helpers.

**Rationale:** Crunch release evidence is valuable because it does not treat arbitrary JSON as proof. Aspen should preserve that discipline even before signing/hashing receipts.

**Alternative:** Emit ad-hoc log JSON. Rejected because it would be hard to compare, validate, or evolve.

**Implementation:** Receipt constructors/tests validate non-empty identities, max stage count, max artifact count per stage, and failure/status consistency.

### 3. String timestamps for the first slice

**Choice:** Store timestamps as RFC3339 strings in the model instead of adding chrono serde coupling now.

**Rationale:** Dogfood already depends on chrono, but the first receipt slice should avoid dependency churn and keep serialization explicit.

**Alternative:** `DateTime<Utc>`. Deferred until live writers need timestamp helpers.

## Risks / Trade-offs

**Receipt overfitting** → Mitigate with a minimal stage/artifact/failure vocabulary and optional identifiers.

**Model without writers** → Mitigate by keeping follow-up tasks in this OpenSpec change for live receipt writing and final summary output.

**Non-canonical pretty JSON** → Mitigate with a compact deterministic helper for hashable bytes and keep pretty output as a later operator convenience if needed.
