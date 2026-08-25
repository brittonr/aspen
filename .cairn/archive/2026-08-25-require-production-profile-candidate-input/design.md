# Design

## Context

The profile is a pure Nickel value, but its checked-in source-gate array currently contains a placeholder. The release validator rejects that value later, while profile export still succeeds.

## Decision

Add an undefined, contracted `candidate_source_ref` field to the production profile. Nickel customize mode must supply this field during export. The profile uses the same field as its only source-gate input.

Strengthen the shared profile contract with a candidate source predicate. It accepts canonical lowercase BLAKE3 references and rejects the all-zero, repeated-`a`, and repeated-`f` dummy references.

The positive fixture explicitly merges a deterministic fixture candidate. The profile template also uses that named fixture value so unrelated negative cases fail for their intended reason.

## Shell and Core Boundary

Nickel owns pure profile construction and contracts. Nix owns fixture invocation and expected failure checks. Operators own the candidate reference and review its evidence before export.

## Non-Claims

A supplied content reference does not prove source identity, evidence truth, freshness, source-gate success, deployment success, or release eligibility.

## Validation

Run the three focused Nickel/Nix checks before and after the change. Record the pre-existing contract-export drift failure separately. Then run strict Cairn gates and the broad Molten Nix rail.
