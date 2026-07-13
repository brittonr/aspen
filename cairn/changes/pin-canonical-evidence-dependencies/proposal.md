## Why

Aspen's root manifest includes SSH Git dependencies without immutable revision pins, while the lockfile can resolve a different source state than reviewers infer from the manifest. Aspen also consumes the older Octet-hosted `valence-core`, so its evidence semantics depend on the duplicate package implementation rather than canonical standalone Valence.

Aspen may remain AGPL. The remediation is to make source identity and distribution posture explicit and reproducible, not to relicense the project.

## What Changes

- Replace floating release dependencies with immutable source revisions and require manifest/lock agreement.
- Migrate `valence-core` consumption to the canonical standalone Valence source after Octet completes its cutover.
- Add dependency-graph and lock-drift checks that reject duplicate canonical Valence package identity.
- Record the AGPL distribution profile, notices, and project-required corresponding-source evidence without treating AGPL as a release blocker.
- Add positive exact-pin fixtures and negative floating-source, lock-drift, wrong-Valence-source, and missing-distribution-evidence fixtures.
- Add a checked-in CI workflow whose verification scope is `nix flake check`.

## Impact

- **Files**: Cargo manifest and lockfile, Nix source pins, dependency policy/checks, release documentation, fixtures, Cairn project spec, and CI workflow.
- **Dependencies**: implementation waits for archived Valence integrity hardening and Octet standalone cutover receipts.
- **Licensing**: AGPL remains an accepted project distribution model; this package makes the selected model and release evidence reviewable.
- **Claims**: exact pins and distribution records prove reviewed source selection and project-policy evidence only, not upstream correctness, legal compliance in every jurisdiction, or release eligibility outside configured policy.
