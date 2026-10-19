# Admitted Raft placement member count preflight verification

## Scope and failing-before reproduction

The isolated branch `change/preflight-raft-placement-members-20260929` starts from clean canonical Aspen `81baf80cb667a22277be22c140d34829e5c7da7f`. Its public producer lives in `src/raft/control/parts/plane/p008/body.rs`, owner regression in `tests/m000/p003/body.rs`. The detached composition has the same producer in `p008` and integrates the new test into its existing `p007` test module, preserving the previous 65 Cairn packages. Internal production CLI usage only parses placement reports; a separate compiled public caller exercises the producer.

Before the production edit, `CARGO_TARGET_DIR=/home/brittonr/.cargo-target/aspen-raft-placement-owner-20260929 nix develop --command cargo test -p molten --lib consensus_placement_member_count_preflight -- --test-threads=1` failed at `33 admitted members must reject before diagnostics`: the old API returned `ConsensusPlacementReport { decision: "deny", admitted_members: [...] }` instead of rejecting the member count. Earlier exact-32 pass, duplicate-entry pass and malformed candidate/admitted reference error assertions in that same test had succeeded before the one-past-bound failure.

## Bounded public behavior

Both source layouts now check `input.admitted_members.len() <= MAX_RAFT_MEMBERS = 32` just after `validate_group_id`, before generic reference walks, placement diagnostics, value serialization or cloning into a report. The 33rd valid admitted ref returns `consensus placement admitted members count 33 exceeds maximum 32`. Existing canonical candidate/admitted ref validation precedence for admitted lists, report shape, exactly 32 pass and existing duplicate-entry pass stay unchanged. `candidate_members` is a candidate pool, not actual admitted group membership: its existing generic 4,096-ref bound still allows 33 candidates with 32 admitted.

After the fix the isolated and composed focused regressions each passed **1/1**, and their `quorum::tests::` filters each passed **18/18**. An independently compiled disposable external package called public `molten::raft_control_plane::consensus_placement_report` and exercised exactly 32 pass, duplicate behavior, malformed candidate/admitted errors, 33 admitted rejection and 33 candidate eligibility; stdout was `public facade: 32 pass; duplicate and invalid-ref precedence retained; 33 admitted denied; 33 candidates eligible`. Its temporary source was removed after execution.

## Scoped validation and static boundary

- `nix develop --command cargo fmt --package molten --check` passed in both worktrees.
- `nix develop --command cargo clippy -p molten --lib -- -D warnings` passed in both worktrees.
- Isolated, nonincremental `cargo octet check --artifact-dir /tmp/molten-raft-placement-owner-lib-20260929 -- -p molten --lib` returned warning-only, **835 warnings**, zero errors/autofixable; its summary has no `p008/body.rs` rows.
- Composed, nonincremental `cargo octet check --artifact-dir /tmp/molten-raft-placement-composition-lib-20260929 -- -p molten --lib` returned warning-only, **1,493 warnings**, zero errors/autofixable. It still reports `unbounded_collection_growth` at `src/raft/control/parts/plane/p008/body.rs:247` despite the entry guard. Runtime bound proof does not imply static clearance.
- `nix develop --command cairn validate --root . --policy cairn-policy/generated/cairn-policy.json --strict` returned empty issue lists and `valid: true` for **53** isolated and **66** composed changes. Structural validation is not release acceptance.

No candidate-membership cap, accepted spec, waiver, policy, baseline, pin or retry of the unchanged known-failing full strict Octet gate was introduced.
