# Design: upgrade drain state proof

## Scope

This change proves upgrade session drain and cutover state-machine behavior. It covers task planning, affected protocol refs, compatibility refs, protocol lifecycle gates, terminal session-state refs, task completion receipts, rewrite/apply evidence, and cutover side-effect boundaries.

## Proof checklist

- **Proof claim**: drain tasks complete and cutover only when a passing protocol-session gate for the affected old protocol binds non-empty terminal state refs; missing, stale, denied, or wrong-protocol evidence denies before mutation.
- **Out of scope**: live traffic quiescence outside recorded protocol lifecycle evidence.
- **Trusted assumptions**: protocol session gates correctly replay accepted operation receipts.
- **Positive evidence**: an upgrade drain trace binds plan, task, from/to refs, protocol gate, terminal states, completion receipt, and cutover receipt.
- **Negative evidence**: missing gate, denied gate, wrong protocol ref, stale compatibility ref, empty terminal refs, missing task evidence, and no-mutation denial.
- **Canonical refs**: upgrade plan/session refs, task refs, from/to protocol refs, compatibility refs, protocol gate refs, terminal state refs, rewrite/apply refs, and cutover receipt refs.
- **Regeneration command**: `cargo test upgrade protocol rewrite`.

## Functional core

Represent drain readiness as a pure predicate over task state, affected refs, and protocol gate receipt fields. Mutation shells cannot write cutover artifacts unless the predicate passes.

## Non-goals

- No automatic authority or transport trust from upgrade receipts.
- No new protocol projection semantics.
