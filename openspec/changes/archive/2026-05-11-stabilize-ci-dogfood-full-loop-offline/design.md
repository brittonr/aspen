## Context

Research from the current focused failure:

- Focused command: `nix build .#checks.x86_64-linux.ci-dogfood-full-loop-test --no-link -L`.
- Failing derivation: `/nix/store/3ip63zr7b9hbi3901c7nba8qbh1a02nx-vm-test-run-ci-dogfood-full-loop.drv`.
- The VM boots Aspen, creates the Forge repository, pushes the sample project, and starts the CI pipeline.
- `format-check` succeeds as a shell job.
- `syntax-check` starts as a `ci_nix_build` job against `.#checks.x86_64-linux.cargo-check`.
- The job fails because the sample flake has `inputs.nixpkgs.url = "nixpkgs"` and no `flake.lock`; guest `nix build` attempts public registry resolution and cannot resolve `channels.nixos.org`.

This matches an earlier class already fixed in `multi-node-dogfood-test`: acceptance fixtures should not depend on in-guest Nix registry/network availability when the rail is supposed to prove Aspen orchestration.

## Goals / Non-Goals

**Goals:**

- Keep `ci-dogfood-full-loop-test` as a deterministic flake-check rail.
- Preserve the three-stage CI pipeline proof and artifact execution checks.
- Make inner Nix input resolution local/offline-safe.
- Add evidence that registry/DNS lock-update failures no longer appear in job logs.

**Non-Goals:**

- Do not promote full dogfood/self-hosting acceptance from this focused VM test alone.
- Do not require public network or substituter access inside the test guest.
- Do not rewrite the CI orchestrator unless fixture determinism and feature wiring are proven insufficient.
- Do not re-enable or claim unrelated unproven rails such as `multi-node-blob-test` or microVM Nix cluster proof.

## Decisions

### 1. Treat `nixpkgs` registry lookup as fixture debt

**Choice:** The failing `inputs.nixpkgs.url = "nixpkgs"` sample flake is the primary seam. Replace it with an explicit local/store-backed input strategy or remove the external input requirement from the fixture.

**Rationale:** The job reaches CI orchestration and fails during flake input resolution. A sandboxed flake-check rail should not depend on `channels.nixos.org`.

**Rejected alternative:** Configure guest global registry alone and leave the sample flake lockless. The failure shows lock update still tries public registry resolution through the checked-out flake path, so the fixture should carry deterministic inputs itself.

### 2. Preserve full-loop semantics while simplifying dependency surface

**Choice:** The fixture may use a local `builtins.derivation`, copied store path, or explicit path input if it still produces distinct check/package outputs and a runnable artifact for the test to inspect.

**Rationale:** The rail's purpose is CI orchestration, stage ordering, and artifact handling. Building a full Rust crate through public `nixpkgs` is incidental and makes the test less deterministic.

**Rejected alternative:** Keep `rustPlatform.buildRustPackage` as the only acceptable fixture shape. That proves more of Nixpkgs/Cargo availability than the CI full-loop contract requires.

### 3. Keep feature wiring explicit

**Choice:** `ci-dogfood-full-loop-test` should continue to use an `aspen-node` package whose feature list includes the Nix job execution surface (`snix`, `snix-build`, and `nix-cli-fallback` when the fallback subprocess path is expected).

**Rationale:** The prior failure class was `native build not available and nix-cli-fallback feature is disabled`; fixture determinism should not mask missing CI binary features.

## Risks / Trade-offs

**Fixture too small to prove useful behavior** → Keep three outputs and artifact execution/inspection so the rail still proves stage ordering, build output, and log/artifact retrieval.

**Overclaiming acceptance** → Report the focused VM rail as flake-check evidence only; require fresh full `nix flake check -L` and a separate dogfood receipt before self-hosting acceptance is promoted.

**Native Snix path coverage regresses** → Preserve log assertions or evidence that the Nix job executor attempts the configured native/fallback path, and keep feature checks in `flake.nix` or focused Nix evals.

## Validation Plan

1. Capture the current failing log excerpt as negative evidence.
2. Patch the sample flake/input strategy so guest jobs do not attempt public registry or DNS resolution.
3. Run focused `ci-dogfood-full-loop-test` and save pass/fail evidence.
4. Inspect job logs for absence of `channels.nixos.org`/flake-registry fetch attempts and presence of successful staged jobs.
5. Run `git diff --check`, `scripts/test-harness.sh export`, and `scripts/test-harness.sh check`.
6. Run fresh full `nix flake check -L` before promoting broader acceptance language.
