# Design: lifecycle receipt determinism proof

## Scope

This change covers canonical lifecycle transition values and lifecycle receipt values. It proves stable hashing and evidence binding for identical inputs and fail-closed behavior when receipt contents drift.

## Proof checklist

- **Proof claim**: identical lifecycle transition inputs produce identical transition refs and receipt refs; any semantic input drift changes the transition ref or resulting receipt evidence.
- **Out of scope**: cryptographic proof beyond BLAKE3 content-ref integrity and canonical Preserves rendering.
- **Trusted assumptions**: canonical hash and Preserves rendering remain deterministic for equivalent values.
- **Positive evidence**: repeated receipt construction for the same input is byte/ref stable.
- **Negative evidence**: changed state, action, cause, refs, supervisor ref, or logical step changes the transition ref or denial evidence as appropriate; tampered receipt values fail validation when a validator/parser is present.
- **Canonical refs**: `transition_ref` is the canonical hash of the transition value, and `receipt_ref` is the canonical hash of the receipt value.
- **Regeneration command**: `cargo test lifecycle`.

## Validator shape

If receipt parsing or validation is missing, implementation should add a pure validator that accepts in-memory Preserves values and returns structured pass/deny diagnostics. File reads and CLI presentation remain outside the core.

## Non-goals

- No replacement of BLAKE3 content refs.
- No network or ledger lookup requirement for local receipt validation.
