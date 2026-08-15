## Phase 1: Dependency source contract

- [x] [serial] Define normalized manifest, lockfile, and Nix source rows for release dependencies. r[molten.project.reproducible_dependencies.contract]
- [x] [serial] Pin every Git dependency in the release closure to an immutable revision and synchronize lock/source identities. r[molten.project.reproducible_dependencies.exact_pins]
- [x] [serial] Add pure drift validation with shell-owned manifest, lockfile, and Nix input loading. r[molten.project.reproducible_dependencies.drift_validation]

## Phase 2: Canonical Valence migration

- [x] [serial] Require archived Valence integrity-hardening and Octet standalone-cutover receipts before selecting the canonical source. r[molten.project.reproducible_dependencies.cross_repo_dependencies]
- [x] [serial] Replace the Octet-hosted `valence-core` dependency with the exact accepted standalone Valence revision. r[molten.project.reproducible_dependencies.canonical_valence]
- [x] [serial] Reject duplicate canonical Valence package identities in the resolved dependency graph. r[molten.project.reproducible_dependencies.unique_valence_identity]

## Phase 3: AGPL distribution profile

- [x] [serial] Define a typed AGPL-allowed release-distribution profile with license, notices, source coordinate, revision, and project-required source evidence. r[molten.project.agpl_distribution_profile.contract]
- [x] [parallel] Document that AGPL is accepted and that distribution evidence does not constitute legal advice or universal compliance. r[molten.project.agpl_distribution_profile.docs]

## Phase 4: Fixtures and verification

- [x] [parallel] Add positive fixtures for exact pins, synchronized lock/source identities, canonical Valence, and complete AGPL distribution evidence. r[molten.project.reproducible_dependencies.fixtures.positive]
- [x] [parallel] Add negative fixtures for floating sources, lock drift, unsupported fetch policy, Octet-hosted Valence, duplicate package identity, and missing distribution evidence. r[molten.project.reproducible_dependencies.fixtures.negative]
- [x] [serial] Run focused positive and negative dependency and distribution-profile tests. r[molten.project.reproducible_dependencies.final_validation]
  - PASS: focused Rust tests, strict Clippy, Nickel positive/negative fixtures, the repository shell, license boundary, compiled Nix profile check, contract-export drift gate, and canonical Cairn proposal/design/tasks gates pass. The local Nix signing-key path is absent, so Nix checks use `--option secret-key-files ''` without changing evaluated derivations.
- [x] [serial] Add a checked-in CI workflow that runs only `nix flake check`. r[molten.project.reproducible_dependencies.flake_check_ci]
- [x] [serial] Run `nix flake check` and Cairn validation/gates before sync and archive. r[molten.project.reproducible_dependencies.final_validation]
  - PASS: full workspace tests and strict Clippy pass; pure `nix flake check -L --option secret-key-files ''` reports `all checks passed`; canonical Cairn validation and the proposal, design, and tasks gates pass before sync.
