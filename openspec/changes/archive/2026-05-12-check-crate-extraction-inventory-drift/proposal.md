# Add deterministic crate-extraction inventory drift checks

## Why

Crate-extraction readiness can pass while the human-facing inventory drifts from the typed Nickel policy and family manifests. That lets stale owners, manifest links, readiness states, or next actions survive review even after evidence is captured.

## What changes

- Extend `scripts/check-crate-extraction-readiness.rs` to validate the selected family against `docs/crate-extraction.md`.
- Fail when the inventory omits the family row, links the wrong manifest, drops the selected policy owner, omits current readiness state, or repeats completed first-blocker language for ready candidates.
- Preserve existing per-family manifest, dependency, and evidence checks.

## Impact

- Reviewers get a deterministic receipt that typed policy, family manifests, evidence, and the broader inventory agree.
- Future readiness promotions are less likely to leave stale public tracking text behind.
