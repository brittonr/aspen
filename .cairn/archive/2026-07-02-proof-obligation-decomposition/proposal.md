## Why

Large evidence claims are hard to review because validation, canonicalization, admission, mutation, replay, and negative behavior can be collapsed into one broad pass receipt. Decomposed proof obligations make each claim smaller, explicitly scoped, and independently testable.

## What Changes

- Add a proof-obligation manifest model for multi-step proof claims.
- Split workflow proofs into input validation, canonicalization, admission, mutation/no-mutation, replay/determinism, and fail-closed obligations.
- Link child obligation receipts to an aggregate proof receipt without granting authority.
- Add Hegel RS properties for obligation ordering, ref stability, and missing-child denial.

## Impact

- **Files**: testing harness proof/report core, evidence-gate aggregation, docs.
- **Testing**: positive aggregate fixture, missing-child and mismatched-child negative fixtures, Hegel RS generated obligation graphs.
