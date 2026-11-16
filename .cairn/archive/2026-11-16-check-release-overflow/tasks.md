## Phase 1: Implementation

- [x] [serial] Enable release overflow checks for the Molten-owned crates in the Cargo profile and the Nix crate overrides, with a flake evaluation check. r[molten.project.release_overflow_checks]
- [x] [serial] Make the Raft quorum-index search and the SpaceWasm deallocation accounting total. r[molten.project.release_overflow_checks]
- [x] [serial] Test the maximum-commit-index transition and a mismatched SpaceWasm deallocation; record the decision; run focused tests, formatting, Clippy and source/lifecycle gates before review, sync and archive. r[molten.project.release_overflow_checks]
