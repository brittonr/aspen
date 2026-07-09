## Why

Evidence chains, local ledger persistence, and artifact registry behavior are tightly related but should not be one ownership domain. Evidence establishes canonical facts and non-claims; ledgers persist artifacts and indexes; registries classify and discover artifacts. Mixing them risks treating storage presence or registry discovery as trust.

## What Changes

- Separate evidence model/verification, ledger persistence, and registry/catalog classification boundaries.
- Keep evidence constructors and parsers independent from local filesystem or Redb storage.
- Make registry discovery read-only and evidence-only unless separate admission authorizes side effects.
- Add positive and negative tests for evidence/ledger/registry confusion cases.

## Impact

Artifact evidence remains canonical, while storage and discovery surfaces become easier to swap, test, and reason about.
