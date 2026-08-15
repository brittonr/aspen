# Design: Hegel core proof properties

## Scope

This change records the generated-property proof laws expected of core evidence logic. It targets pure cores and deterministic shells with explicit inputs. It does not replace concrete fixtures, CLI integration tests, or release receipts.

## Initial laws

The initial Hegel RS proof laws cover canonical ref stability, traceability decision equivalence, deny monotonicity when stale evidence is added, diagnostic evidence not satisfying pass gates, replay comparing canonical refs rather than rendered logs, and persisted counterexamples using canonical fixture data.

## Generated inputs

Generators should use bounded collections, named constants for bounds in Rust code, and explicit schemas for refs, requirement ids, receipt ids, decisions, and diagnostics. Negative generation should mutate one semantic binding at a time where possible so failures are readable.

## Counterexamples

When a shrunk case is persisted or enters a proof artifact, it should be canonical Preserves fixture data with seed, shrink path, input, expected law, and receipt refs.

## Non-goals

- No random ambient proof state.
- No replacement for exact negative fixtures.
