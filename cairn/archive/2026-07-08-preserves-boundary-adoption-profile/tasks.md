## Tasks

- [x] [serial] Define the Preserves boundary adoption profile row contract. r[molten.runtime_spine.preserves_boundary_profile.contract]
- [x] [serial] Add positive fixtures for canonical node control, ticket, workflow bundle, receipt, and evidence envelopes. r[molten.runtime_spine.preserves_boundary_profile.fixtures.positive]
- [x] [serial] Add negative fixtures for non-canonical bytes, missing schema labels, stale BLAKE3 refs, and raw-Preserves core coupling. r[molten.runtime_spine.preserves_boundary_profile.fixtures.negative]
- [x] [serial] Implement pure profile validation and a thin artifact-measuring shell. r[molten.runtime_spine.preserves_boundary_profile.validation]
- [x] [serial] Document adapter-only boundaries and non-claims. r[molten.runtime_spine.preserves_boundary_profile.docs]
- [x] [serial] Run focused checks and Cairn validation/gates. r[molten.runtime_spine.preserves_boundary_profile.final_validation]

Implementation evidence: `molten_core::preserves_profile` validates parsed rows and artifact measurements; `docs/preserves-boundary-profile/valid.ncl` exports; `non-canonical.ncl`, `missing-schema-label.ncl`, `stale-ref.ncl`, and `raw-core-coupling.ncl` fail closed; `docs/modularity-boundaries.md` documents adapter-only boundaries and non-claims. Checks passed: `cargo test -p molten-core`, `cargo test --lib`, `cargo fmt --check`, pre-commit, Nickel fixtures, and Cairn validation.
