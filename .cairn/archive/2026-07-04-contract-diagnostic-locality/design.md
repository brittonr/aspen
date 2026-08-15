# Design: Contract diagnostic locality

## Context

A single large predicate can be easy to write but hard to debug. As contract surfaces grow stricter, validation output should make failures actionable without relaxing the contract.

## Refactor shape

Prefer layers:

1. field-level contracts for simple domains such as refs, ids, enums, positive integers, safe paths, and non-empty arrays.
2. small named pure predicates for cross-field invariants such as distinct layout dirs, coherent resource relationships, unique descriptors, and resolved internal references.
3. fixture names and expected-failure metadata that map each negative case to one invariant.

## Validation evidence

The smallest useful validation evidence is a positive/negative fixture run showing that valid fixtures still export and invalid fixtures still fail with the expected failure class. Rust admission checks remain separate when generated Preserves or JSON artifacts are involved.

## Boundary

This change is about observability of validation failures. It does not add new authority, runtime Nickel evaluation, or acceptance of ambiguous inputs.
