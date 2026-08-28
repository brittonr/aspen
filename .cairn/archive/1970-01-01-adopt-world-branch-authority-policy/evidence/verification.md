# Verification

Date: 2026-08-28

## Completed boundary

Promotion-gated branch authority now consumes Molten-owned release-reservation admission.

`PromotionReservationPort` has one observation method and no dispatch method. The shell adapter checks these facts:

- one exact promotion plan;
- the complete committed reservation set;
- one selected reservation that matches the plan;
- the candidate head and release branch class; and
- the promotion result still states that external effects are incomplete.

The pure authority core binds the promotion plan, reservation, candidate, capability, and authority plan into one BLAKE3 admission identity.

It denies missing, incomplete, crossed, uncommitted, or dispatch-authorizing observations. The receipt records metadata identities only.

## Reviewed dependency cohort

- Basalt: `89675cd4f585f837323c049e4a25f7b94c903038`.
- Archived Molten promotion boundary: `0d9dfe4ba9008f3dab1a3c14d5470e8af21a1f4a`.
- Transactional Reconciliation Core: `eb2bd3441753af97bfcb247cef7cc22d72675b62`.

Nix generated the merged lock file. It advanced `basalt-src` to the reviewed revision and removed the obsolete `ucan-src` input.

## Positive and negative behavior

Positive fixtures cover all seven closed policy modes. Promotion-gated activation now admits only an exact committed reservation observation.

Negative fixtures cover these promotion cases:

- a missing admission;
- an incomplete reservation set;
- an uncommitted reservation;
- a crossed candidate;
- a reservation that does not match its plan; and
- a dispatch-authority overclaim.

The promotion runtime uses no destination-grant, simulation, transfer, or effect-dispatch path.

## Rust verification

The final source passed these checks:

- `cargo fmt --all -- --check`;
- 5 focused authority-core tests;
- 5 focused authority-shell tests;
- 1,390 complete `molten` library tests;
- 68 binary tests and 61 CLI integration tests;
- 300 complete `molten-core` tests and 7 doctests;
- all remaining workspace and integration tests; and
- 5 `molten-release-policy` binary tests.

`cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.

## Octet

`checks.x86_64-linux.world-authority-octet-deny-all` passed with zero findings, warnings, and errors.

The first strict run found a long model file and two sentinel-fallback findings. The final source moves promotion DTOs into a focused module and propagates conversion failures.

## Generated plans

The repository-owned `unit2nix` command generated both plans twice with identical bytes.

- `build-plan.json`: 713 crates, 4 workspace members, 989 build units, and 1,019 test units.
- `release-policy-build-plan.json`: 254 crates, 2 workspace members, 323 build units, and 2 roots.
- The release plan contains the `molten-release-policy` binary target.
- Cargo lock SHA-256: `4831149e509872705a47b22aeb5e48e06a2eb7b15524fc09ee7cbf766ffcf935`.
- Main-plan BLAKE3: `9c54d0a376b678e6adb9fa8034290600d28dc186e61090167937395bd67cb25e`.
- Release-plan BLAKE3: `38c05946c2ef44213e1a1b7ec4449870cdb96c097a86b3a5c41827c43c73c987`.

SHA-256 appears only because the unit2nix interoperability field requires it.

## Nix and lifecycle

These focused checks passed:

- `world-authority-octet-deny-all`;
- `world-authority-dependency-identity`;
- `world-authority-schema-inventory`;
- `world-promotion-dependency-identity`;
- `release-dependency-profile`;
- `release-profile-validation`; and
- `deterministic-drift-gate`.

`nix flake check --no-build -L` evaluated every compatible output and reported `all checks passed`.

Current Cairn validation and proposal, design, and tasks gates passed with the shared generated policy.

## Inherited repository rail

The inherited `contract-export-drift-gate` remains outside this change. A full `nix flake check -L` pass is not claimed.

## Non-claims

Reservation admission does not authorize effect dispatch. It does not prove handler success, effect meaning, deployment success, future enforcement, or release eligibility.

Policy decisions still do not mint, move, activate, store, or enforce capabilities.
