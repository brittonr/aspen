# Proposal: Admit a bounded record pattern subset

## Why

`RuntimePattern` supports two forms: an exact canonical value and a single wildcard binding
(`src/runtime/predicates/parts/mod/p000/body.rs`). The accepted requirement
`molten.runtime_spine.preserves_patterns` defers "record, array, dictionary, conjunction, negation, or extensible
compound matching" to a future admitted extension.

The manual gives the design for the record case: a group pattern checks the type of its input, matches declared key
positions in increasing Preserves order, and succeeds when the candidate has *more* structure than the pattern requires
(`38-protocols__syndicate__dataspacePatterns.md → Group`, `→ Example`). That rule is what allows protocol extension,
because a record with extra fields keeps matching an older subscription. Today every added field breaks an exact
subscriber.

The same pattern form is dangerous near identity. A match that ignores extra fields must never decide a canonical value
ref or an identity comparison. This package therefore admits the pattern for routing only and keeps identity exact.

## What Changes

- Admit one bounded record pattern form: a record label, declared key positions, and nested wildcard bindings, with a
  deterministic visit order and ordered bindings. r[molten.runtime_spine.record_pattern_subset]
- Match with the at-least rule: undeclared keys in the candidate are ignored, a missing or mistyped declared key fails
  the match, and a form outside the admitted subset denies before it controls routing.
- Keep identity exact. Identity, receipt, and canonical-ref computations MUST NOT consume pattern bindings or admit a
  record through a pattern match.
- Require a named consumer before implementation starts, so the extension lands against a real subscription and not
  against speculation.

## Impact

- **Files**: `src/runtime/predicates/parts/mod/p000/body.rs`, `src/runtime/dataspace/state.rs`,
  `src/runtime/dataspace/tests.rs`, `docs/architecture.md` dataspace section.
- **Testing**: positive at-least match on a record with an extra field; negative tests for a missing declared key, a
  wrong type at a declared key, an undeclared pattern form, and an unbounded pattern; an identity non-interference test
  that shows routing admits a record while its canonical ref is unchanged.
- **Non-goals**: no array, dictionary, negation, or conjunction patterns, no match-order ambiguity rules, no change to
  canonical value identity, and no new authority.
