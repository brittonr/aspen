## Why

Issue #51: many receipts carry `<checks [...]>` lists of constant `pass` tuples, including denial labels such as `("commitment-mismatch", "pass")`, and consumers verify only that a required name is present. The lists add little beyond the decision field but read like independent verification.

## What Changes

- The receipt check-list contract states that check lists are producer-described labels: a required-name parser establishes record shape only, and admission uses the decision, exact references, current authority and validated observations. Families that compute statuses say so, and consumers that rely on a computed status check its value.
- The shared and family check helpers document that they verify names only.
- Consumer tests prove that a denial receipt carrying every required label with `pass` stays denied, and that the name-only parsers accept a wrong status, so the boundary is explicit rather than assumed.

No receipt bytes change, so canonical identities and frozen fixtures are unaffected.

## Impact

- **Files**: `docs/architecture.md`, `src/preserves/parts/rail/p004/body.rs`, the rewrite, catalog and schema check helpers, the secrets and node-runtime producers' comments, and focused tests.
- **Testing**: the new consumer tests plus the existing preserves-rail, secrets, rewrite and catalog suites.
- **Owner / consumer**: the Preserves rail toolkit owns the check-set helpers; every receipt consumer reads the clarified contract.
