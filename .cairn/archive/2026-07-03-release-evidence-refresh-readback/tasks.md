# Tasks: release-evidence-refresh-readback

## Phase 1: Candidate validation

- [x] [serial] r[molten.release_evidence_refresh.current_candidate_matrix] Confirm `cargo fmt --check`, `cargo test`, and `cargo clippy --all-targets -- -D warnings` are green for the candidate tree.
- [x] [serial] r[molten.release_evidence_refresh.current_candidate_matrix] Build hermetic nextest evidence with `nix build .#checks.x86_64-linux.nextest`.

## Phase 2: Dogfood and release evidence

- [x] [serial] r[molten.release_evidence_refresh.dogfood_readback] Build `nix build .#checks.x86_64-linux.dogfood-local-node` after nextest evidence is current.
- [x] [serial] r[molten.release_evidence_refresh.bundle_graph] Verify release evidence bundle members, signed members, promotion, signed promotion where available, summary, export manifest, archive, and export verification all refer to the same candidate evidence graph.

## Phase 3: Documentation and denial evidence

- [x] [parallel] r[molten.release_evidence_refresh.stale_denial] Record stale/missing/tampered release-member denial coverage from existing tests or run explicit verification commands if current output paths expose new stale cases.
- [x] [serial] r[molten.release_evidence_refresh.docs] Update README/operator notes with current evidence paths and caveats only after the graph passes.
