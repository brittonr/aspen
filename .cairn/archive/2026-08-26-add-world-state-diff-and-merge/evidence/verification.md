# World diff and merge verification

Verified on 2026-08-26 in the dedicated Molten worktree.

## Dependency cohort

- Choregraph branch history: `b3e08e19750f53bdbcae970cdf58a47a791ed20b`
- Schema Identity Core: `2562c8aa38a034061f9af9f3e17280494a5b8de2`
- Schema Migration Core: `4fe90e130f2871cf69a6febcdc70785adca98aea`

The external Schema Migration Core baseline passed sixteen focused tests. Molten world-commit, world-head, and typed-storage baselines passed.

`nix build path:$PWD#checks.x86_64-linux.world-merge-dependency-identity -L --builders ''` passed.

## Focused behavior

- `cargo test -p molten-core world_merge --all-features`: 7 passed.
- `cargo test -p molten world_merge --all-features`: 8 passed across the shell and CLI parser.
- Core and shell Clippy checks passed for all targets and features with warnings denied.

Positive cases cover equal roots, ancestor replacement, disjoint keyed values, exact migrations, pure handlers, publish-last roots, and conflict artifacts.

Negative cases cover missing or ambiguous bases, duplicate sources, incompatible schemas, absent migrations, runtime-sensitive roots, handler effects, bounds, conflicts, and failed publication.

The standalone `world-merge merge-publish` command remains fail-closed. It requires composed authority, migration, and handler adapters.

## Strict Octet

`nix build path:$PWD#checks.x86_64-linux.world-merge-octet-deny-all -L --builders ''` compiled the real world-commit, world-head, and world-merge core sources.

- Status: `clean`
- Findings: `0`
- Warnings: `0`
- Errors: `0`
- Config: `b3:0ed5032eb7fbce2d3eb2a5c57ad4d2751fc13840dd563e8d0448b72903bcae40`
- Profile: `b3:7c9defd01ac2d9afff7408d4ca38280df0407c148e8e77d298d8fe1a8d594bf3`

The focused result does not convert unrelated repository-wide findings into a pass.

## Nix and lifecycle

- Both unit2nix plans were regenerated with the pinned tool.
- `release-dependency-profile` passed with the exact schema-migration row.
- `release-profile-validation` passed with receipt `blake3:d54663d8d304af6d5f486ead64a51c06d74e1ab6320f62b2c6e9864dfe5f33c6`.
- `nix flake check --no-build path:$PWD --builders ''` passed.
- Strict Cairn validation and `git diff --check` passed.

These checks do not prove migration correctness, handler correctness, application merge semantics, branch movement, remote convergence, effect release, or release eligibility.
