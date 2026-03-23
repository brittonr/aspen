## Context

snix-eval has full implementations of `builtins.fetchurl` and `builtins.fetchTarball` but `builtins.fetchGit` is a stub returning `NotImplemented`. The `Fetch` enum has a `Git()` variant with `todo!()` in `Debug::fmt`, `store_path()`, and `ingest()`. This has been the case across all known snix revisions.

Separately, Aspen's `flake_lock.rs` resolver handles `github`, `gitlab`, `tarball`, and `path` input types but explicitly returns `Unsupported` for `"git"` inputs.

The fix needs two layers because there are two eval paths that encounter git inputs:

1. **call-flake.nix path** (`evaluate_flake_derivation`): Parses flake.lock in Rust, resolves inputs to local paths before evaluation. Git inputs fail at the resolver level.
2. **flake-compat path** (`evaluate_flake_via_compat`): Evaluates flake-compat.nix through snix-eval, which calls `builtins.fetchGit` for `type = "git"` locked inputs. Fails at the eval level.

Both layers use the `git` CLI via subprocess, following the same pattern as `curl` in `fetch.rs`. The `git` binary is always available in Nix build environments.

### Current data flow for a github input (working)

```
flake.lock: { "type": "github", "owner": "NixOS", "repo": "nixpkgs", "rev": "abc..." }
     │
     ├─ call-flake.nix path: flake_lock.rs → fetch_github_input() → curl tarball → unpack → outPath override
     │
     └─ flake-compat path: flake-compat.nix → fetchTarball { url = "https://api.github.com/..." } → snix-eval handles it
```

### Target data flow for a git input

```
flake.lock: { "type": "git", "url": "https://spectrum-os.org/git/spectrum", "rev": "abc..." }
     │
     ├─ call-flake.nix path: flake_lock.rs → fetch_git_input() → git clone --bare → git archive → unpack → outPath override
     │
     └─ flake-compat path: flake-compat.nix → fetchGit { url = "..."; rev = "..."; } → snix-glue handles it
```

## Goals / Non-Goals

**Goals:**

- Implement `builtins.fetchGit` in snix-glue matching Nix's argument set and return attrset
- Add `fetch_git_input()` to `flake_lock.rs` for the call-flake.nix eval path
- Verify content via narHash from flake.lock
- Cache cloned repos to avoid redundant fetches
- Support the parameters flake-compat actually passes: `url`, `rev`, `ref`, `submodules`
- Unblock Aspen's self-build through the zero-subprocess pipeline

**Non-Goals:**

- `verifyCommit` / GPG signature verification (experimental in Nix, not used by any Aspen input)
- `lfs` support (not used by any Aspen input)
- `exportIgnore` / `.gitattributes` filtering (edge case, not required for flake inputs)
- Pure in-process git (using `gix`/`git2` crate instead of CLI) — added complexity for no practical gain here
- Upstreaming to snix (separate effort, do it later once the implementation is battle-tested)

## Decisions

### 1. Use `git` CLI, not `gix`/`git2` crate

**Decision**: Shell out to `git` via `std::process::Command`.

**Rationale**: The `git` binary is always present in Nix build environments. `gix` is already in the dependency tree (used by Forge) but adding it as a dependency to snix-glue creates coupling. The `curl` subprocess pattern in `fetch.rs` is proven and works well. The git operations needed are simple: `clone --bare`, `fetch`, `archive`, `rev-parse`, `log --format`.

**Alternative considered**: Use `gix` crate for in-process git. Rejected because: adds significant dependency to snix-glue, complicates the authentication story (SSH keys, credential helpers), and doesn't match snix's existing subprocess patterns for fetchers.

### 2. fetchGit returns an attrset, not a string — new pattern needed

**Decision**: `builtin_fetch_git` constructs and returns a `Value::Attrs` directly, bypassing the `fetch_lazy` helper.

**Rationale**: `fetchurl` and `fetchTarball` return a string (store path) and use `fetch_lazy` which calls `Fetch::store_path()` for lazy evaluation. `fetchGit` returns an attrset `{ outPath, rev, shortRev, lastModified, revCount, narHash, submodules }`. The `Fetch::store_path()` mechanism can still compute the store path when `narHash` is known upfront, but the return value construction is different.

The implementation:

1. Parse arguments from the attrset
2. If `narHash` is provided, compute store path via `build_ca_path` (same as `Fetch::store_path`)
3. If store path exists locally (checked via `PathInfoService`), skip fetch and build return attrset from cached metadata
4. Otherwise: clone/fetch, ingest into castore, compute NAR hash, persist PathInfo, build return attrset

### 3. Bare repo cache at a deterministic path

**Decision**: Cache bare repos at `$TMPDIR/aspen-git-cache-<pid>/<sha256(url)>` (per-process) or `$XDG_CACHE_HOME/aspen/git/<sha256(url)>` (persistent across evaluations).

**Rationale**: Nix uses `~/.cache/nix/gitv3/<sha256(url)>[-shallow]`. We follow a similar scheme. For CI builds (which are the primary use case), per-process caching is sufficient since each CI job evaluates once. For interactive use, persistent caching avoids re-cloning on every eval.

**Implementation**: Start with per-process cache via `FetchCache`. The git-specific code manages the bare repo lifecycle; `FetchCache` manages the extracted working tree keyed by narHash.

### 4. Two-phase approach: Fetch::Git variant + builtin body

**Decision**: Fill in `Fetch::Git { url, rev, r#ref, shallow, submodules, all_refs, exp_nar_sha256 }` in the enum, implement `store_path()` and `ingest()`, but have `builtin_fetch_git` do additional work to construct the attrset return value.

**Rationale**: `store_path()` can compute the CA store path when `exp_nar_sha256` is known (same pattern as `Fetch::Tarball`). `ingest()` handles the actual clone + archive + castore ingestion. The builtin function wraps these with argument parsing and attrset construction.

### 5. Manage snix-glue changes as a patch

**Decision**: Apply changes to snix-glue as a cargo patch in Aspen's workspace `Cargo.toml`, pointing to a local directory or forked git ref.

**Rationale**: snix is referenced via tarball URL in `flake.nix` and git checkout in `Cargo.toml`. The cleanest approach is to maintain a patch branch. This avoids vendoring all of snix into Aspen's tree. The patch surface is small: ~200 lines in `fetchers.rs` and `fetchers/mod.rs`.

**Alternative considered**: Vendoring snix-glue into Aspen's tree. Rejected because snix has many internal dependencies and vendoring one crate means vendoring the whole workspace.

## Risks / Trade-offs

**[Git CLI availability]** → The `git` binary must be in `$PATH` during evaluation. In NixOS test VMs and CI builders, `git` is always available via the `nix` profile. Add a clear error message if `git` is not found, pointing to the `git` package.

**[Submodule recursion depth]** → Malicious repos could have deeply nested submodules. Mitigation: enforce `MAX_SUBMODULE_DEPTH = 10` and `MAX_CLONE_SIZE` limits. Match Tiger Style principles.

**[Network access during eval]** → `fetchGit` requires network access to clone repos. For locked flake inputs, the rev is pinned so the fetch is reproducible. For unlocked inputs (no rev), the result depends on the remote state — this matches Nix's behavior. In the call-flake.nix path, all inputs are locked.

**[narHash mismatch]** → If the cloned content doesn't match the expected narHash from flake.lock, the fetch fails loudly. This is the correct behavior — it means the lock file is stale or the remote history was rewritten.

**[Patch maintenance]** → snix-glue patches must be rebased when upgrading snix. The patch is small and isolated to fetcher code. Track upstream `fetchGit` progress; if snix implements it natively, drop the patch.

**[Authentication]** → Private git repos need SSH keys or credentials. The `git` CLI handles this via the user's SSH agent or credential helpers, same as Nix. No special handling needed in Aspen — if `git clone <url>` works from the command line, it works during eval.
