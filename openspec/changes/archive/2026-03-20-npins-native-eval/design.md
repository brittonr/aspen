## Context

The native snix-build pipeline (landed in `LocalStoreBuildService`) handles everything from derivation parsing through sandbox execution to output ingestion — all in-process. The single remaining subprocess is `nix eval --raw <flake>.drvPath`, needed because snix-eval doesn't implement flake lock resolution.

npins projects don't use flakes. Their `default.nix` calls `builtins.fetchTarball` and `builtins.fetchGit` with pinned hashes. `NixEvaluator` already wires up snix-eval with snix-glue's fetcher builtins (`fetchTarball` ✓, `fetchurl` ✓). `fetchGit` is not yet implemented in snix-glue — it returns `NotImplemented`. This limits us to npins projects using tarball-based pins (GitHub, GitLab, Channel types).

## Goals / Non-Goals

**Goals:**

- Evaluate npins-based projects to `.drvPath` using snix-eval in-process, zero subprocesses
- Prove the pipeline end-to-end with a VM test: snix-eval → parse .drv → LocalStoreBuildService → ingest → upload
- Keep the `nix eval` subprocess path as fallback for flake projects and npins projects that use `fetchGit`

**Non-Goals:**

- Implementing `fetchGit` in snix-glue (upstream work)
- Migrating Aspen's own build from flakes to npins (future work, not this change)
- Supporting nixtamal or niv (same pattern, can add later)

## Decisions

### 1. Detect project type by directory contents

Check for `npins/sources.json` in the project root. If present and the `snix-eval` feature is enabled, use the in-process eval path. Otherwise fall through to `nix eval` subprocess.

Alternative: explicit `eval_mode` field in the CI job payload. Rejected because detection is reliable and doesn't require payload changes.

### 2. Evaluate to drvPath via `NixEvaluator::evaluate_with_store()`

Construct the Nix expression:

```nix
let
  sources = import <project_dir>/npins;
  pkgs = import sources.<nixpkgs_pin> {};
in (pkgs.<attribute>).drvPath
```

The `<nixpkgs_pin>` name and `<attribute>` come from the CI job payload. For the test, these are hardcoded. In production, the payload specifies them (or we default to `nixpkgs` + the flake attribute syntax).

Alternative: evaluate the entire `default.nix` and walk the result. Rejected because directly selecting the attribute is simpler and matches how `nix eval` works.

### 3. snix-eval runs on a blocking thread

`snix_eval::Evaluation::evaluate()` is synchronous and CPU-bound. Run it via `tokio::task::spawn_blocking` to avoid blocking the async runtime. The result (a `Value`) is extracted as a string (the drv path).

### 4. Test with a self-contained npins project

The VM test creates a minimal npins project with a pre-populated `sources.json` pointing to a channel pin. The derivation is trivial (`/bin/sh -c "echo hello > $out"`). No network access needed — the channel URL is fetched at eval time by snix's `fetchTarball` builtin, but for the test we use a derivation that doesn't need nixpkgs at all (same as the existing `snix-native-build` test). This keeps the test fast and offline.

For the simplest proof: the npins project's `default.nix` evaluates to an attrset, and the attribute we select is a derivation whose `.drvPath` we extract. No nixpkgs needed — just a `derivation { ... }` literal.

## Risks / Trade-offs

- **[snix-eval completeness]** snix-eval may not handle every Nix language construct perfectly. → Mitigation: the test uses a trivial expression; complex projects fall back to `nix eval` subprocess. The fallback is already tested and working.
- **[fetchGit missing]** npins projects with plain git pins fail at eval time. → Mitigation: catch `NotImplemented` error and fall back to subprocess path. Log which builtin was missing so users know why native eval was skipped.
- **[eval-time fetching]** `builtins.fetchTarball` in snix-eval fetches at eval time (network I/O during eval). → Mitigation: npins pins include hashes, so the fetch is content-addressed and cacheable. For the test, we avoid network fetches by using a derivation that doesn't import nixpkgs.
