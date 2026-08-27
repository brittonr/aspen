# World branch-head verification

Verified on 2026-08-26 in the dedicated Molten worktree.

## Dependency cohort

- Choregraph RID: `rad://zL2ncTUeASVYwcoGkEXv9JKgGbAF`
- Choregraph revision: `b3e08e19750f53bdbcae970cdf58a47a791ed20b`
- Cargo transport: `https://seed.radicle.garden/zL2ncTUeASVYwcoGkEXv9JKgGbAF.git`
- Artifact Auth revision: `c932138d880ddf4c2967f4c024b489b5c0022bf1`

`nix build path:$PWD#checks.x86_64-linux.world-head-dependency-identity -L --builders ''` passed.

The Choregraph history baseline passed seven tests. The Artifact Auth baseline passed nine core, conformance, and consumer-mapping tests. Both reruns used local Nix builders.

## Focused behavior

- `cargo test -p molten-core world_head --all-features`: 8 passed.
- `cargo test -p molten world_head --all-features`: 9 passed across the shell and CLI parser.
- Core and shell Clippy checks passed for all targets and features with warnings denied.
- Positive coverage includes creation, linear advance, merge ancestry, threshold authentication, stable plans, atomic restart, and conflict storage.
- Negative coverage includes stale or skipped generations, replay, unrelated history, signer failures, denied authority, uncertain storage, and rollback overclaims.

The standalone `world-head advance` command remains fail-closed. A composed `WorldHeadAuthorityPort` is required for mutation.

## Strict Octet

`nix build path:$PWD#checks.x86_64-linux.world-head-octet-deny-all -L --builders ''` compiled the real world-commit and world-head core sources.

- Status: `clean`
- Findings: `0`
- Warnings: `0`
- Errors: `0`
- Config: `b3:eac5aeb42df6841715be6f5aded75a2706ab6142d57b737f563960c9e87e5d80`
- Profile: `b3:b0caab84d17ba659e29137736e33df7dce8c1b9c14a2ff31e1d875169d2a9434`

This focused result does not convert unrelated repository-wide findings into a pass.

## Nix and lifecycle

- Both unit2nix plans were regenerated with the pinned tool.
- The release dependency profile accepts exact private Radicle development rows and rejects a false HTTPS classification.
- `release-dependency-profile` passed.
- `release-profile-validation` passed with receipt `blake3:d54663d8d304af6d5f486ead64a51c06d74e1ab6320f62b2c6e9864dfe5f33c6`.
- `nix flake check --no-build path:$PWD --builders ''` passed.
- Strict Cairn validation and `git diff --check` passed.

These checks do not prove whole-store rollback detection, distributed consensus, remote convergence, merge correctness, effect release, or release eligibility.
