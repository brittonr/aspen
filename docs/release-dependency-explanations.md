# Release dependency explanations

The release-dependency validator emits a stable `rule_id` and bounded
`supporting_facts` for each diagnostic. The selected explanation families are
`LockDrift` and `MissingArchiveReceipt`; other diagnostics carry their rule
identity and an empty support set until a concrete operator question justifies
more facts.

Canonical report material and admission remain unchanged: the canonical rows
still use the diagnostic code, subject, and message. The structured fields are
an explanation view, not a second decision source.

## Snapshot-bound explanations

`molten_core::release_dependency::explanation` adds an occurrence-exact view of
the same two families. `explain_release_dependencies` takes one immutable
`ReleaseDependencyInput`, the declared collection state of each observation
relation (dependency, registry, archive), and explicit limits. It evaluates
every snapshot fresh; nothing is cached between snapshots.

Each explanation names a refined rule
(`release-dependency/lock-drift/git-revision`,
`release-dependency/missing-archive-receipt/observation-absent`, and so on) and
exactly two support references. A reference is either a field of one row
occurrence (relation, index, field, value, where a missing value stays
`missing`) or an absence of a key in a relation declared completed.

- A missing dependency observation stays the validator's `MissingSourceRow`; it
  never produces a `LockDrift` explanation. A present observation with a missing
  lock field produces `LockDrift` support whose observed value is `missing`.
- An absent archive observation (`observation-absent`, depth 2) and an
  observation reporting `archive_present=false` (`reported-absent`, depth 1)
  remain separate rules of `MissingArchiveReceipt`.
- Duplicate observations keep the validator's semantics: the last occurrence
  with a key is the one compared, and support names that occurrence. Duplicate
  expectations each get their own explanation.
- Rows join only on their own key (`manifest_dependency` or receipt id), so
  facts of two packages are never combined, even when one package's values
  equal the other's expectations.

The report is bound to a BLAKE3 snapshot identity (domain-separated,
length-framed fields in input order, so row order and duplicates are part of
the identity), the rule revision `molten.release-dependency.explanation.v1`, the
declared collections, and the limits. `verify_explanation_references` checks
displayed explanations against a snapshot without trusting the evaluator: a
replaced snapshot is `StaleSnapshot`, and a forged, cross-package,
non-effective duplicate, contradicted absence, absence over an uncompleted
relation, or support that does not satisfy the rule body is rejected.

## Outcomes

| Outcome | Meaning | Report | Release pass |
| --- | --- | --- | --- |
| `Completed` | Every derived explanation is shown. | yes | only if the same-snapshot validator admits |
| `Truncated` | Evaluation completed; the output bound omitted `omitted_explanations` rows. | yes | only if the same-snapshot validator admits |
| `Unavailable` / `Incomplete` | An observation relation was not completely collected. | no | never |
| `Unsupported` | Rows exceed the validator bounds, or limits are zero or above their caps. | no | never |
| `BudgetExhausted` | The fact, step, or depth budget was exceeded. | no | never |

Caps: 4,096 steps, 512 projected facts, depth 2, and 32,768 rendered output
bytes. Only a relation declared completed can support an absence; a completed
snapshot with a known missing observation still yields its explanation.

## Comparison with a relational pilot

The comparison subject is a minimal Datalog-style evaluator compiled for tests
only (`explanation/relational_pilot.rs`). It projects the snapshot into
set-valued relations whose first column is the row occurrence, derives
superseded and effective observations with stratified negation, and evaluates
the five drift rules and two archive rules by naive bottom-up nested-loop joins
to a fixpoint. Its derived heads become the same `Explanation` shape.

Both paths run over one fixture corpus (`explanation/corpus.rs`): valid input,
lock source drift, lock revision drift plus a missing lock field, an absent
dependency observation, an absent archive observation, an archive reported
absent, duplicate observations with the clean or the drifted occurrence last,
duplicate archive observations, a duplicate expectation, crossed packages, and
registry checksum drift. The comparison tests assert that both paths produce
identical explanations, that their `(code, subject)` rows equal the validator's
selected rows, and that the pilot's references pass the independent verifier.
Corrected and removed snapshots are covered by the replacement tests.

Source-measured implementation size (non-blank, non-comment lines at the
change's source revision):

| Part | Lines |
| --- | --- |
| Structured Rust derivation (`explanation.rs`, step meter through archive rule) | 164 |
| Relational pilot (projection, rules, evaluator, head conversion) | 456 |
| Shared by both: types, outcomes, limits (`explanation.rs` before the derivation) | 454 |
| Shared by both: snapshot identity (`identity.rs`) | 212 |
| Shared by both: reference verification (`verify.rs`) | 241 |

Findings asserted by the tests:

- Usefulness is equal: the pilot can only reach parity by carrying the same
  occurrence columns, last-occurrence strata, and support values the Rust path
  reads directly. Today's textual diagnostics name the code, subject, and
  values, but not the row occurrence or the snapshot.
- The pilot's absence rule is guarded by a `completed` fact; without it the
  rule derives nothing and reports nothing. The explicit
  `Incomplete`/`Unavailable` refusal has to live outside the rules either way.
- Neither path adds a dependency. A production engine would add a crate to the
  Cargo and Nix pins, both unit2nix plans, and license review.

Projection and evaluation steps, materialized facts and text bytes, iterations,
and wall time are captured by the change's comparison run and retained under
`.cairn/archive/2026-11-16-explain-release-dependency-snapshots/evidence/` with the exact
commands, toolchain, and source identity.

## Decision

**Keep structured Rust explanations; stop Datalog engine integration for these
families.** The two selected families are a direct field comparison and one
indexed absence check over a completed relation. The relational form needs the
same occurrence, ordering, completeness, and budget machinery plus a rule
evaluator about 2.8 times the size of the Rust derivation, while producing the
same explanations. The cost measurement can reverse this only if it shows the
pilot cheaper; the keep/stop record in the change evidence states the measured
result. Add a rule family only with a named release-operator question and a
completed-snapshot comparison.

## Boundaries

The explanation is diagnostic. It does not grant authority, change release
eligibility, prove legal compliance, or replace the owned-package license
checker. Rule identities are stable text for review and tests, not immutable
cryptographic identities. `molten-core` performs no repository, network,
process, or clock effects for it; wall time is measured only by the test runner.
