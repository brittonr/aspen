# Verification

Date: 2026-08-28

## Dependency disposition

The former Weft blocker is no longer a valid dependency.

- Weft revision `dee51eff9940bc53921bd8675b68c5abce8b05dd` withdraws its runtime, replay, and product-neutral effect-runtime plans.
- Choregraph revision `b3e08e19750f53bdbcae970cdf58a47a791ed20b` owns immutable branchable history. It emits no effect outcomes or dispatch authority.
- Molten therefore retains ordered effect-log validation and current effect authority.

The source review used three serial lenses because subagent consent was absent. The lenses checked implementation ownership, lifecycle state, and consumer authority.

| Approach | Result | Evidence |
|---|---|---|
| Wait for `weft-replay` | Falsified | Weft marks the plan as superseded before implementation. |
| Use Choregraph for effect outcomes | Rejected | Choregraph owns history and explicitly excludes effect outcome and dispatch meaning. |
| Keep validation in Molten | Validated | Existing effect-log checks and current world-transition traces cover the required boundary. |

## Implemented boundary

`plan_world_promotion_observation_commit` binds one acknowledged observation to one logical `recorded-effect` transition.

The plan binds the promoted candidate, exact reservation, exact observation, logical profile, and explicit successor. It denies these inputs:

- an uncommitted reservation;
- an unacknowledged or missing observation;
- a reservation mismatch;
- an unchanged successor;
- a malformed schema reference or byte bound; and
- any invalid world-transition trace.

The plan does not mutate the promoted commit. It does not grant dispatch authority or claim opaque replay equivalence.

Promotion-specific effect-log fixtures accept exact ordered request and outcome bindings. They reject missing outcomes, mismatches, and live fallback.

## Rust verification

The final source passed these checks:

- `cargo fmt --all -- --check`;
- 8 focused `molten-core` promotion tests;
- 8 focused `molten` promotion tests;
- 2 promotion CLI parser tests;
- 4 existing ordered effect-log tests;
- 295 complete `molten-core` tests and 7 doctests;
- 1,385 complete `molten` library tests;
- 63 binary tests and 61 CLI integration tests;
- all remaining workspace and integration tests; and
- 5 `molten-release-policy` binary tests.

`cargo clippy -p molten-core -p molten --all-targets --all-features -- -D warnings` passed.

## Octet

`checks.x86_64-linux.world-promotion-octet-deny-all` passed with zero findings, warnings, and errors.

The first strict run found one long function and one long test file. The final source splits validation, trace construction, and observation tests.

## Generated plans

The repository-owned `unit2nix` command generated both plans twice with identical bytes.

- `build-plan.json`: 644 crates, 4 workspace members, 904 build units, and 934 test units.
- `release-policy-build-plan.json`: 54 crates, 2 workspace members, 71 build units, and 2 roots.
- The release plan contains the `molten-release-policy` binary target.
- Cargo lock SHA-256: `e3e55a68c9d06182e02239c6abd991b1576b9c25625b8114b6cdc4b09210c71c`.
- Main-plan BLAKE3: `4148dd59e770950866561593cd8f4d6e24c7dd92a0ab06e1a40f141647c5320f`.
- Release-plan BLAKE3: `a75a2f80085b08f1a5bfea7a455a5085479ce8407a8392a99c825455bf74a1b6`.

SHA-256 appears only because the unit2nix interoperability field requires it.

## Nix and lifecycle

These focused Nix checks passed:

- `world-promotion-dependency-identity`;
- `world-promotion-octet-deny-all`;
- `release-dependency-profile`;
- `release-profile-validation`; and
- `deterministic-drift-gate`.

`nix flake check --no-build -L` evaluated every compatible output. It reported `all checks passed` and omitted incompatible systems.

Current Cairn validation and proposal, design, and tasks gates passed with the shared generated policy.

## Inherited repository rail

`contract-export-drift-gate` still exits with status 1. The checked export differs from the current policy export across inherited lifecycle fields.

This change does not edit the Cairn policy source or generated policy file. A full `nix flake check -L` pass is not claimed.

## Non-claims

This evidence does not prove external effect success, generic exactly-once execution, opaque replay equivalence, deployment success, or release eligibility.

Promotion commits local eligibility only. Unknown outcomes remain uncertain until an admitted observation resolves them.
