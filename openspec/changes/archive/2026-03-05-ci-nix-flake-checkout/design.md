## Context

CI checkout (in `crates/aspen-ci/src/checkout.rs`) extracts files from Forge git objects into a bare directory — no `.git/`. Nix flakes require git context to evaluate. The `prepare_for_ci_build()` step already patches `.cargo/config.toml` and `Cargo.lock` for CI, but doesn't address the git requirement. Meanwhile, the SNIX cache upload path in `NixBuildWorker` is dead code because `snix_*_service` fields are always `None` in the node binary.

## Goals / Non-Goals

**Goals:**

- Make `nix build .#pkg` work in CI checkout directories
- Enable SNIX cache uploads from CI builds
- Expose `publish_to_cache` in pipeline config

**Non-Goals:**

- Full git history in checkouts (shallow single-commit is sufficient)
- Authenticated Forge push (separate change)
- Bootstrap automation script (separate change)

## Decisions

### Decision 1: `git init` + `git add` + `git commit` after checkout

Initialize a minimal git repo in the checkout directory after `prepare_for_ci_build()` completes. This is ~20 lines in `checkout.rs`.

**Why not `path:.` flake URL:** `path:` flakes don't resolve `flake.lock` inputs properly — they can't fetch locked git dependencies. Standard `.` flake URL requires git context.

**Why not full git clone from Forge:** Massive complexity. Would need to implement git pack protocol on top of Forge's BLAKE3-native storage. The checkout already has all files; we just need to wrap them in a git commit.

### Decision 2: Inject commit hash as git rev

Use the original Forge commit hash (hex) in the git commit message and tag, so `nix flake metadata` and build logs show the actual source revision. The git commit won't match the Forge BLAKE3 hash, but the message provides traceability.

### Decision 3: Wire SNIX services from router.rs pattern into NixBuildWorkerConfig

The SNIX services (`IrohBlobService`, `RaftDirectoryService`, `RaftPathInfoService`) are already constructed in `setup/router.rs` for the gRPC castore server. Create them the same way in `setup/client.rs` and pass to `NixBuildWorkerConfig`. Gate behind `#[cfg(feature = "snix")]` and `config.snix.is_enabled`.

### Decision 4: Add `publish_to_cache` to JobConfig, default true

Add the field to the Nickel schema and `JobConfig` struct. The `NixBuildPayload` already has `publish_to_cache` — just plumb it from config. Default `true` so existing configs get cache publishing automatically.

## Risks / Trade-offs

**[Risk]** `git` binary not available in CI execution environment → builds fail at init step.
→ Mitigation: `git` is already in `dogfood-node.nix` environment. Add a pre-flight check in `prepare_for_ci_build()` that warns if git is missing.

**[Risk]** `git init` + `git add -A` on large checkouts is slow.
→ Mitigation: Bounded by existing 500MB / 50K file limits. A 50K file `git add` takes ~2s — acceptable for CI.

**[Risk]** `flake.lock` references git revisions that aren't in the shallow repo.
→ Mitigation: Nix fetches locked inputs independently via their URLs. The local git repo just needs to contain `flake.nix` and `flake.lock` — Nix doesn't resolve input refs from the local history.

## Open Questions

None — all decisions are straightforward with clear rationale.
