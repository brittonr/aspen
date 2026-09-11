# Design: Admit a bounded record pattern subset

## Context

Pattern matching happens in two places: dataspace routing for `Observe` subscriptions, and the reference harness. Both
consume `RuntimePattern`, which validates its form and rejects unknown forms. Binding output is already ordered and
bounded by an existing binding-size limit, so the extension adds one form rather than a new matching engine.

## Approach

Add one variant to the pattern type: `Record { label, entries }`, where each entry is a declared key position plus a
sub-pattern. Evaluate as follows.

- Check the input type first. A non-record input fails the match.
- Visit declared positions in increasing Preserves order, which the manual requires for stable numbered bindings
  (`38-protocols__syndicate__dataspacePatterns.md`, note 2).
- Fail when a declared position is absent or its value does not match the sub-pattern.
- Ignore candidate keys that the pattern does not declare, which is the at-least rule.
- Append bindings in visit order, with the speculative-rollback rule the manual gives: a failed sub-pattern discards
  bindings it pushed (`38-protocols__syndicate__dataspacePatterns.md → Binding`).

Bound the admitted form explicitly: maximum record depth, maximum declared entries, and the existing binding byte
limit. A pattern over the bound denies at admission, before it reaches routing.

## Decisions

### Decision: Routing only, identity never

**Choice:** Pattern matches route observations. They do not compute identity, they do not feed receipt hashing, and
they do not widen a value's acceptance in an identity path.

**Rationale:** At-least matching is forward compatible by construction and therefore unsuitable where two nodes must
agree on one canonical value. Keeping identity exact preserves the current acceptance rule that unsupported compound
patterns deny.

### Decision: One form, one package

**Choice:** Admit only the record form here.

**Rationale:** The manual's group patterns also cover sequences and dictionaries. Each adds matching cases and bounds.
Admitting one form with its own tests keeps the review small and leaves the remaining forms to their own change.

## Risks / Trade-offs

- An at-least match can hide a producer error, because an extra or renamed field still matches. Tests pin the declared
  positions, and the negative cases cover a renamed declared field.
- Routing cost grows with declared entries times subscription count. Declared entries and depth are bounded at
  admission.
