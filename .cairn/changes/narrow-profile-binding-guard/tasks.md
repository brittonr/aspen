## Phase 1: Guard

- [x] [serial] Accept exactly the reviewed `test-fixtures` root feature set in the profile-binding guard and keep every other root feature set denied. r[molten.dependency_audit.dev_feature_review]
  - `cargo -Zscript tools/dependency-audit.rs --self-test` passes, including the new positive control for `["test-fixtures"]` and the negative control for `["test-fixtures", "spacewasm-pilot"]`. The real resolve agrees: `cargo metadata --filter-platform x86_64-unknown-linux-gnu` reports the root `molten` node with `features = ["test-fixtures"]` and no other root node carries features, so the previous rule could never pass.

## Phase 2: Verification

- [ ] [serial] Re-run the pinned audit at one RustSec revision and record that the `profile-binding` finding is gone and every other class is unchanged. r[molten.dependency_audit.dev_feature_review]
- [x] [serial] Run the repository gates and Cairn validation, then sync and archive with this evidence. r[molten.dependency_audit.dev_feature_review]
  - Cairn 0.2.1 validated this change from the store with `--strict` (0 issues) and the proposal, design and tasks gates each reported 0 issues; the audit re-run stays open because the Nix flake app cannot be rebuilt while the store client is wedged.
