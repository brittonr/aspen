# Design: Bind metadata-free git source hashes

## Context

The flake builds crates from unit2nix plans. Some git sources are replaced by reviewed flake inputs through
`localGitSources`. The rest, ChaosControl, durable-authority-state, bounded-http, content-identity, and Choregraph,
are fetched with `pkgs.fetchgit` using the plan's `sha256`. unit2nix applies `crate-hashes.json` entries first and
prefetches only the sources those entries do not cover.

## Decisions

### Decision: Bind every git source through revision-qualified `crate-hashes.json`

**Choice:** Record one `url?rev=<rev>#<crate>@<version>` entry per git source and revision, with the SRI NAR hash of the
checkout without `.git` (submodules included, the same as `pkgs.fetchgit`).

**Rationale:** unit2nix already supports this contract, so this needs no fork and no pin bump. Revision-qualified keys
cannot silently apply to a later revision. Also binding the sources that `localGitSources` overrides keeps both plans
free of `.git`-derived hashes and makes the check uniform.

### Decision: Enforce the binding in a pure flake check

**Choice:** `scripts/git-source-hashes.sh --check` runs without network or git. It compares Cargo.lock, flake.lock,
`crate-hashes.json`, and both plans. The `git-source-hash-binding` check runs it on the flake source.

**Rationale:** Any regeneration that lets unit2nix prefetch again changes a plan hash away from the bound value, and
the check fails on every builder, not only on builders that lack stale store paths.

### Decision: flake.lock narHash as the unreachable-host fallback

**Choice:** If `nix-prefetch-git` cannot reach a host, `--write` uses the `narHash` of a flake input locked at the same
revision, but only when that input does not fetch submodules. `--check` also cross-checks those entries.

**Rationale:** Nix computed that hash over the same revision without `.git`. The prefetched values for Artifact,
Basalt, and Kamacite match their flake.lock values exactly.

## No-spec classification

Accepted requirement text does not change. Semantic review inputs: `crate-hashes.json`, both generated plans,
`scripts/git-source-hashes.sh`, the `git-source-hash-binding` and dev shell hunks in `flake.nix`, and
`docs/reproducible-dependencies.md` step 6.

## Failure behavior

`--write` fails closed when a source can be neither prefetched nor matched to a locked input. `--check` and the flake
check fail and name each unbound source, each non-SRI or unqualified key, and each plan hash that drifted.

## Risks / Trade-offs

- `workspaceRoot` in the regenerated plans records the generating worktree path. That matches existing unit2nix
  behavior, and the Nix build does not use it.
- The optional `flux` profiler source is bound even though no plan uses it, so that enabling the feature cannot fall
  back to prefetch.
