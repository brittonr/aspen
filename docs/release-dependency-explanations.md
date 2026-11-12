# Release dependency explanations

The release-dependency validator now emits a stable `rule_id` and bounded
`supporting_facts` for each diagnostic. The selected explanation families are
`LockDrift` and `MissingArchiveReceipt`; other diagnostics carry their rule
identity and an empty support set until a concrete operator question justifies
more facts.

Canonical report material and admission remain unchanged: the canonical rows
still use the diagnostic code, subject, and message. The structured fields are
an explanation view, not a second decision source.

## Selected cases

- `LockDrift` support names the expectation and observation field values that
  differed or were missing.
- `MissingArchiveReceipt` support distinguishes an absent archive observation
  from a present observation that reports `archive_present=false`.

Facts are formed from the same completed `ReleaseDependencyInput` snapshot as
the validator. A missing observation is named as `missing`; it is not silently
treated as a value.

## Evaluation result

**Decision: keep structured Rust diagnostics; stop Datalog engine integration
for these cases.**

The selected diagnostics are a direct field comparison and one indexed absence
join. The existing validator already sorts and deduplicates the complete
diagnostic set. A Datalog pilot would need a separate projection that preserves
row occurrence identity, an explicit completed-snapshot boundary for absence,
budget accounting, fact-reference validation, and a comparison harness. That
adds maintenance and deployment surface without changing the selected rows or
their admission result.

The structured Rust view gives the operator the useful part of the proposed
explanation: which rule fired, which expectation and observation facts support
it, and whether the observation was absent. The experiment does not claim that
Datalog is unsuitable for a future, genuinely recursive policy family. Add such
a family only with a named operator question and a completed-snapshot
comparison.

## Boundaries

The explanation is diagnostic. It does not grant authority, change release
eligibility, prove legal compliance, or replace the owned-package license
checker. Rule identities are stable text for review and tests, not immutable
cryptographic identities.
