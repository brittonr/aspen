# Replay input and divergence reference
Mode: Reference

This reference identifies the objects to request from a replay producer and the code that interprets them. It covers deterministic fixture comparison, effect-log validation, and identity freshness. It does not define a new interchange schema. Canonical Preserves values and BLAKE3 references determine identity; Rust structs are API carriers, not an alternative identity format. All details are source-checked; no commands or runtime scenarios were executed for this guide.

## Input families and owners

| Input | Owner | What the consumer does | Important limit |
|---|---|---|---|
| `deterministic-fixture-record-v1` | Fixture builder and parser, replay parts p001/p011 | Checks embedded identity/effect-log hashes; extracts journal refs and bound output/state refs | Parser selects the first journal |
| `ReplayTraceSummary` | Summary API, p013 | Validates reference shapes and bounded vectors, then compares summaries | Caller materializes the complete ordered summary |
| `EffectLogValidationInput` | Effect validator, p006/p007 | Compares recorded entries with supplied consumption observations | Does not observe operating-system effects |
| `ReplayFreshnessInput` | Freshness API, p009 | Compares expected and evidence identity components | Does not discover missing dependencies |
| Harness report value | `src/harness/replay.rs` | Validates report evidence or replays its embedded suite, depending on API | Not accepted as a fixture record |

The distinction matters when naming a failure. A fixture parser error is not a first-divergence receipt. A successful summary comparison is not proof that the reported effect consumption was faithful.

## Summary and boundary fields

The [summary implementation](../../../src/deterministic/parts/replay/p013/body.rs) owns these fields:

| Carrier | Fields | Review use |
|---|---|---|
| `ReplayTraceSummary` context | `run_identity_ref`, `handler_profile_ref` | Identify intended context; perform freshness separately |
| Summary aggregates | `turn_refs`, `effect_log_refs`, `output_refs`, `final_state_ref` | Bind ordered aggregate evidence and the final state |
| `ReplayBoundaryRef` location | `turn_index`, `event_index`, `boundary_kind`, `field_path` | Locate a semantic comparison boundary |
| Boundary labels | Optional `actor_id`, `session_id`, `vat_id` | Associate location with supplied logical identifiers |
| Boundary identity | `boundary_ref` | Compare the canonical object named by that location |

Reference vectors and boundary vectors must be nonempty and are bounded at 1024 items by this API. Boundary refs and kind tokens are validated, and field paths must not be empty. The validator does not establish contiguous turn/event indices or sort boundaries: vector order is supplied by the caller. Keep this distinction when accepting a custom summary producer.

The fixture adapter emits turn index 0 and event indices 0–6: scheduler, input, effect-request, effect-response, policy-decision, action, receipt. Its actor label is `actor:helper`; session and vat labels are absent. Output and final state are compared after boundaries, not inserted as additional fixture boundary rows.

## Effect-log fields and validation order

| Carrier | Real fields | Meaning |
|---|---|---|
| `EffectLogEntry` | `sequence`, `effect_kind`, `run_identity_ref`, `handler_profile_ref`, `turn_ref`, `boundary_ref`, `request_ref`, `response_ref` | Recorded effect binding |
| `ConsumedEffect` | `sequence`, `effect_kind`, `request_ref`, `response_ref`, `boundary_ref`, `used_live_fallback` | Caller-supplied observation of consumption |
| `EffectLogValidationInput` | `expected_run_identity_ref`, `expected_handler_profile_ref`, `entries`, `consumed` | Expected context plus both evidence sets |
| `EffectLogValidation` | `decision`, `validation_ref`, `diagnostics`, `value` | Canonical pass/deny evidence |

Each input slice has a 1024-entry limit. Recorded sequences begin at zero and advance by one. Effect-kind tokens are nonempty lowercase ASCII letters, digits, hyphens, or underscores. Shape/reference failures return an error before receipt construction. Semantic diagnostics are selected in order: identity/profile, recorded sequence, duplicate request, binding mismatch, unconsumed entry, missing recording, live fallback. Current code records the first diagnostic, not every failure.

## Freshness identity components

`ReplayRunIdentity` contains `artifact_ref`, `dependency_closure_ref`, `initial_state_ref`, `schema_refs`, `policy_refs`, `capability_refs`, `revocation_refs`, `handler_profile_ref`, `seed_or_effect_log_ref`, `runtime_refs`, `tool_refs`, and `replay_profile`. The [freshness implementation](../../../src/deterministic/parts/replay/p009/body.rs) hashes supplied identity values and emits expected/evidence identity refs plus the first stale-component diagnostic. Its subject and evidence refs bind the intended reuse question; a pass remains evidence-only.

## Output artifacts and availability

| Artifact | Producer | Retain or inspect |
|---|---|---|
| Fixture verify receipt | `verify_fixture_record_value` | Built-in baseline comparison, divergence kind, output/state bindings |
| Comparison receipt | `compare_replay_summaries` | Expected/actual summary refs, decision, divergence ref, redaction status |
| First-divergence path | Core comparison result's `first_divergence` | Turn/event indices, labels, field path, expected/actual refs, profile, redaction |
| Explain receipt | `explain_replay_comparison_value` | Comparison ref and existing divergence ref; no payload resolution |
| Prefix receipt | `compare_replay_prefix_manifests` | Manifest refs and first mismatch ref; partial-fetch range-receipt requirement |

The fixture CLI's `compare` writes the comparison value, not the separately returned divergence path. Do not assume possession of the receipt implies possession of every referenced object. Prefix comparison accepts supplied manifest metadata; its requirement marker is not evidence that a range fetch was performed.

## Worked interpretation and cautions

For baseline versus `ChangedEffectResponse`, the source tests expect `turns[0].effect-response-ref`; event index 3 identifies that fixture boundary. Expected and actual refs identify different canonical responses, not printable response contents. A path can also report a missing side using `none`. Do not convert `none` into a fabricated content reference.

The current comparison selector has no standalone handler-profile mismatch branch, although profiles are included in canonical summaries. Review profile identity separately rather than interpreting `pass` as equality of every summary field. This is a source-review limitation, not a runtime-reproduced bug. The [technical companion](../../technical/foundations/deterministic-playback-contract.md) explains the broader separation between context, effects, and trace equality.

## Sources

- [Handbook](../README.md)
- [Multi-turn compare/explain contract](../../replay-multiturn-explain.md)
- [Effect-log hardening contract](../../replay-effect-log-hardening.md)
- [Deterministic playback companion](../../technical/foundations/deterministic-playback-contract.md)
- [Effect types and validator](../../../src/deterministic/parts/replay/p006/body.rs)
- [Divergence selection and explain](../../../src/deterministic/parts/replay/p014/body.rs)
- [Path record serialization](../../../src/deterministic/parts/replay/p015/body.rs)
