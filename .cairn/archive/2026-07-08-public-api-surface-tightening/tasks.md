## Tasks

- [x] [serial] r[molten.modularity.public_api.classified_surface] Inventoried root-crate public modules and classified stable APIs, compatibility aliases, internal implementation modules, and generated/test support in `docs/modularity-boundaries.md`.
- [x] [serial] r[molten.modularity.public_api.intentional_exports] Introduced/documented `molten::core_api` and `molten::prelude` as preferred stable surfaces while preserving compatibility aliases.
- [x] [serial] r[molten.modularity.public_api.visibility] Recorded the migration blocker for hiding implementation-only root modules: compatibility aliases intentionally re-export historical paths and removal requires a separate compatibility-evidence change.
- [x] [parallel] r[molten.modularity.public_api.validation] Added positive root compile/use checks for intended prelude API and negative denied-plan checks for accidental effect expansion.
- [x] [serial] r[molten.modularity.public_api.validation] Ran `cargo test -p molten-core`, `cargo test --lib`, `cargo fmt --check`, pre-commit, and Cairn validation.
