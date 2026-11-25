# Runtime spine deltas

## ADDED Requirements

### Requirement: Canonical Preserves identities are collision-free, unique, lowercase-normalized, and traverse Embedded values
r[molten.runtime_spine.canonical_preserves_identity_and_embedded_scan] All canonical identities and receipts MUST be hashed over canonical packed Preserves with domain separation and closed typed fields—never `serde_json::to_vec`, Rust `Debug` (`{:?}`), or unframed delimited strings. Structural scans MUST traverse Preserves `Embedded` (`#:<...>`) values, `Option<ContentRef>` MUST have one unique canonical encoding, and `blake3:` digest validation MUST reject uppercase hex uniformly across `molten-core` and `src/`.

#### Scenario: Embedded payload cannot evade security or content-ref scans
- GIVEN an `<ambient-token>` or `<secret>` wrapped inside a Preserves `Embedded` value (`#:<...>`)
- WHEN `find_ambient_job_token`, `find_sensitive_structural_marker`, or `contains_structural_content_ref` scans the value
- THEN traversal MUST descend into the embedded value and detect the marker.

#### Scenario: Uppercase digest or bare optional content ref denies canonical decoding
- GIVEN a `blake3:` digest containing uppercase `A..=F` or a bare `"blake3:..."` where `<some "blake3:...">` is required
- WHEN canonical decoding or core validation runs
- THEN it MUST reject the non-canonical representation.

### Requirement: Runtime Dataspace enforces monotonic counters, validated recorded RNG, honest rollback receipts, actor-attributed facet turns, and ForceRun precedence
r[molten.runtime_spine.dataspace_determinism_and_rollback_integrity] Dataspace transitions MUST prevent `logical_time` regression and `u64::MAX` overflow, MUST validate recorded `Effect::Random` values against `upper`, MUST NOT emit `TurnCommitted` or `Pass` receipts for rolled-back retractions, MUST attribute facet turns to their owning actor and notify observers on facet-stop retractions, and MUST prioritize `ForceRun` over `DependencyGated` demands.

#### Scenario: Rolled-back retraction does not emit TurnCommitted
- GIVEN an actor attempts to retract an assertion it does not hold
- WHEN `apply_step_with_predicate_receipt` executes
- THEN it MUST record rollback without appending `Event::TurnCommitted` or returning a `Pass` commit receipt.

#### Scenario: Recorded clock regression or out-of-range RNG denies replay
- GIVEN a recorded clock value less than `before.logical_time`, equal to `u64::MAX`, or a recorded random value `>= upper`
- WHEN `recorded_effect_response_transition` runs
- THEN it MUST fail closed with a typed replay error.
