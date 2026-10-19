# Verification

- Before source edit, `cargo test -p molten --lib prolly_snapshot_validates_profile_before_block_read -- --nocapture` failed at the +1 profile assertion: the missing block took precedence over the core profile-bound issue.
- After the preflight, the same targeted test passed (1 passed). It uses `LocalProllyBlockStore` with no staged root for the +1 case, then stages actual encoded blocks and loads the exact-`MAX_GRAPH_FACTS` profile successfully.
- Independently compiled a throwaway public-API caller with `rustc --extern molten --extern molten_core` and ran it against a real capability-rooted `LocalProllyBlockStore`. Output: `external typed LocalProllyBlockStore smoke: +1 rejected before read, exact limit loaded`. The smoke source and binary live outside the repository under the isolated Cargo target directory.
- Isolated owner: `cargo fmt --package molten --check` and `cargo clippy -p molten --lib -- -D warnings` passed; `cargo octet check --artifact-dir ... -- -p molten --lib` reported `warning-only`, 1,534 warnings, zero errors in its scoped `status.json`. The static `unbounded_collection_growth` row remains at `src/prolly_map/service.rs:59`; the early profile validation and real-store tests demonstrate the existing cap on the behavioral path. This is not a full-workspace or strict-ci count.
- Isolated owner `cairn validate`: 56 packages, empty issues. Private composition targeted regression: 1 passed, 1,510 filtered, in a distinct Cargo target directory. Composition `cairn validate`: 61 packages, empty issues.

No accepted spec, existing Cairn package, policy profile, or pin was changed.
