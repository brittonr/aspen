## Context

The no-disabled probe includes smaller warning families that can indicate correctness, portability, or clarity risks. These should be handled differently from mechanical shape lints because changes may alter logic and therefore need tests.

## Design

### Safety-polish boundary

This change owns warning families whose remediation may affect behavior or invariants:

- borrowed argument types;
- platform-dependent casts;
- unbounded collection growth;
- ambiguous boolean naming;
- unchecked division;
- ignored results;
- nested conditionals.

Each slice should start with the smallest focused baseline test for the touched logic. Changes should prefer pure helper cores with explicit input/output contracts, named bounds, checked arithmetic, explicit result handling, and clear branch structure.

### Validation

Every logic-affecting slice must include positive and negative tests for the changed behavior. After focused tests pass, run formatting, Clippy, and a no-disabled Octet probe to record warning movement.

### Non-goals

- Do not bury behavior changes inside broad source-shape refactors.
- Do not silence warnings without a stronger invariant or test.
- Do not use broad allow attributes as burn-down progress.
