# Verification: lifecycle probe result reference intake

## Source boundary and failing-before regression

The bound full F09/F10 Octet catalog recorded 3,615 warning-only findings and 312 critical review rows; it marked F1922/F1923 at `src/lifecycle/parts/mod/p003/body.rs:109`. This production public API accepts a borrowed caller-supplied probe slice and previously cloned each optional status reference into a result vector with no entry-point count or byte cap. The new exact/+1 regression failed before the fix: 1,025 valid refs were admitted. `validate_probe_results` now checks `probes.len() > MAX_REFS` before allocating or cloning, reusing the existing 1,024-reference lifecycle receipt envelope. Each returned status reference is validated as a fixed-width canonical content ref before cloning.

## Behavioral evidence

- The isolated `nix develop --command cargo test -p molten --lib probe_result_status_refs_obey_lifecycle_reference_bound -- --nocapture` passed after the fix: one test, zero failures. It admits exactly 1,024 valid refs, denies the 1,025th before collection growth, and retains evidence-reference and generation rejection for admitted input.
- An independently compiled disposable Rust binary linked against the isolated production `molten` library printed `external lifecycle probe bound: 1024 admitted, 1025 denied` after asserting both cases.
- The same focused regression passed in the 60-change private composition under a worktree-unique `CARGO_TARGET_DIR`: one test, zero failures. A separately compiled binary linked against that worktree's independently built library printed the same 1,024-admitted/1,025-denied result.

## Focused quality and static-scanner limits

Owner `nix develop --command cargo fmt --package molten --check` and `cargo clippy -p molten --all-targets -- -D warnings` passed. Owner scoped `cargo octet check -p molten -- --lib` and `--bin molten` reported warning-only, 1,513/1,559 findings, zero errors and zero autofixable findings. In the 60-change composition, package fmt and all-target Clippy passed; its library Octet scope reported warning-only, 1,472 findings, zero errors and zero autofixable findings. The scanner still flags the guarded push at line 115 in these scopes; no static finding clearance or waiver is claimed. These scoped counts cannot be subtracted from, nor substituted for, the original bound full catalog. Full strict-ci was not rerun.

The first composed Rust checks hit an unrelated full `/tmp` disk quota. Relocating only disposable build temporaries to the spacious cargo-target filesystem allowed the independent composed regression and external API caller to pass; this is not a changed source behavior or a suppressed compiler warning.

## Change-local and disjoint composition

Owner `cairn validate --root . --policy cairn-policy/generated/cairn-policy.json --strict` returned 56 changes and empty issue arrays. Composed validation returned 60 changes and empty issue arrays, preserving the previously composed 59 change packages. Proposal/design/tasks gates were valid, issue-free, advisory in each worktree. Paired SHA-256 hashes matched for lifecycle implementation, regression, and all five change-local metadata/proposal/design/tasks/delta-spec files. `git diff --check` passed in each worktree, and `git diff --exit-code -- .cairn/specs/runtime-spine/spec.md` confirmed no accepted runtime-spine spec edit. No sync, archive, waiver, commit, or pin change is claimed.
