# Partial verification evidence

Date: 2026-08-27

## Verified implementation boundary

The change branch implements and verifies the local world-promotion core and shell:

- canonical promotion, reservation, attempt, observation, and reconciliation records;
- pure admission, stable BLAKE3 identities, complete reservation checks, dispatch planning, retry planning, and reconciliation;
- one Redb transaction for active-head movement, the promotion record, and the complete reservation set;
- external effect dispatch only after reservation claim, current admission, and an `attempting` record;
- observation-first handling for unknown publication and lost acknowledgments;
- explicit retry acknowledgment and new attempt identity without changing the logical reservation identity;
- fail-closed standalone mutation commands;
- exact Transactional Reconciliation Core pin at revision `eb2bd3441753af97bfcb247cef7cc22d72675b62`.

## Passing checks

- `cargo test -p molten-core world_promotion`: 6 passed.
- `cargo test -p molten world_promotion`: 6 passed.
- `cargo test -p molten cli::runtime::worldpromotion`: 2 passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: passed.
- `world-promotion-octet-deny-all`: passed with zero focused findings.
- `world-promotion-dependency-identity`: passed.
- `release-dependency-profile`: passed.
- `release-profile-validation`: passed its positive case and expected negative cases.
- `deterministic-drift-gate`: passed after unit2nix regenerated both plans.
- `nix flake check --builders '' --no-build`: all outputs evaluated.
- Strict Cairn validation and proposal, design, and tasks gates passed with the shared Cairn policy.

`build-plan.json` contains 640 crates. `release-policy-build-plan.json` contains 54 crates and retains the `molten-release-policy` binary target. Both plans include Transactional Reconciliation Core at the exact revision above.

## Blocking dependency

The Cairn remains blocked and unarchived. Weft `main` is at `8a89ffa9`. Its README states that no runtime implementation exists. The active `establish-deterministic-interaction-runtime` Cairn still has the `weft-replay` and effect-log extraction tasks unchecked. Therefore, Molten cannot pin a published `weft-replay` revision or run the required old-to-new effect-log and observation-commit compatibility corpus.

This branch does not claim final effect-release compatibility, exactly-once external effects, completed publication, synchronized accepted specifications, archive eligibility, or release readiness.

## Inherited repository rail

A full `nix flake check -L` still reaches the inherited `contract-export-drift-gate` failure because `cairn-policy/default.ncl` does not regenerate the checked-in `cairn-policy/generated/cairn-policy.json`. This change modifies neither file. Focused Nix checks and full Nix evaluation passed.
