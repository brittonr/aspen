## Context

The `flake-eval-native` change (commit `51ade04e0`) added in-process flake evaluation via call-flake.nix + snix-eval. It works end-to-end for flakes whose inputs are already in the local `/nix/store`. When inputs are missing, `resolve_all_inputs()` calls stub fetchers that return `NotFound`, causing `evaluate_flake_derivation()` to fail and `try_native_build()` to fall back to the `nix eval --raw .drvPath` subprocess.

The stub fetchers in `flake_lock.rs`:

- `fetch_github_input()` → always returns `NotFound`
- `fetch_tarball_input()` → always returns `NotFound`
- `fetch_git_input` → doesn't exist, returns `Unsupported`

In CI, inputs are usually present from prior builds, but fresh workers or updated `flake.lock` files trigger the fallback. Implementing real fetching eliminates this subprocess dependency.

## Goals / Non-Goals

**Goals:**

- Fetch GitHub/GitLab archive tarballs and generic tarball inputs over HTTP(S)
- Verify fetched content against `narHash` from flake.lock (integrity check)
- Cache fetched inputs to avoid redundant downloads within a worker's lifetime
- Make `evaluate_flake_derivation()` succeed for standard flake inputs without any subprocess

**Non-Goals:**

- Fetching `git` inputs via `git clone` (complex, involves submodules, sparse checkout). Will return a clear error that triggers subprocess fallback.
- Adding fetched content to the local `/nix/store` (we use temp dirs as `outPath` overrides — call-flake.nix reads them directly).
- Replacing snix-glue's built-in fetchers (those operate during evaluation; ours resolve inputs *before* evaluation starts).
- Supporting `indirect` flake registry inputs (those resolve through the nix registry, which we don't implement).

## Decisions

### 1. Fetch to temp directory, not nix store

Fetched inputs are unpacked to a temp directory under the worker's scratch space, not imported into `/nix/store`. The call-flake.nix expression uses `outPath` overrides that point directly to these directories.

**Rationale**: Adding to `/nix/store` requires NAR packing, signing, and `nix-store --import` or PathInfoService registration. The override approach works today and matches how the evaluator already handles local inputs — it just reads files from the `outPath`.

**Alternative considered**: Import into PathInfoService. More "correct" but significantly more complex and not needed — the eval expression just reads files from the path.

### 2. narHash verification via NAR hash of unpacked directory

After unpacking a tarball, compute the NAR hash of the resulting directory and compare against the `narHash` from flake.lock. This is the same algorithm Nix uses — `nar-sha256` of the directory contents.

**Rationale**: narHash is the content address Nix computes after unpacking. Comparing against the tarball's checksum would be wrong (tarball format differs from NAR format). Use `snix_store::nar` to compute the hash.

**Alternative considered**: Skip verification and trust HTTPS + TLS. Rejected — narHash verification is a core Nix security property, and flake.lock is the source of truth.

### 3. Use ureq (blocking HTTP) not reqwest

The fetch happens inside `resolve_all_inputs()` which is called from `evaluate_flake_derivation()` — a blocking function run via `spawn_blocking`. Using `ureq` (blocking, minimal deps) avoids pulling reqwest's async runtime into a sync context.

**Rationale**: reqwest isn't a workspace dep, requires tokio runtime in scope, and would need `block_on` inside `spawn_blocking`. `ureq` is sync, small, and sufficient for downloading tarballs. If `ureq` isn't acceptable, fall back to `std::process::Command` calling `curl` (always available in nix devshell).

**Alternative considered**: reqwest with `blocking` feature. Larger dep tree. Also considered: just shell out to `curl` + `tar`. Works but couples to system tools.

### 4. In-memory fetch cache keyed by narHash

Cache fetched inputs in a `HashMap<String, PathBuf>` keyed by the SRI narHash string. The cache lives on the `NixEvaluator` (or a shared `FetchCache` struct passed to `resolve_all_inputs`). Multiple builds with the same flake.lock reuse cached unpacked directories.

**Rationale**: Same narHash = same content. The cache persists for the worker's lifetime. Temp directories are cleaned up when the cache is dropped.

### 5. GitHub archive URL format

GitHub archives are fetched via `https://github.com/{owner}/{repo}/archive/{rev}.tar.gz`. GitLab uses `https://{host}/{owner}/{repo}/-/archive/{rev}/{repo}-{rev}.tar.gz`.

These are stable APIs that don't require authentication for public repos. Private repos are out of scope for now (would need token injection).

## Risks / Trade-offs

- **[Network dependency]** → Fetching requires outbound HTTPS. CI workers behind restrictive firewalls may fail. Mitigation: clear error messages, subprocess fallback remains available.
- **[NAR hash computation cost]** → Computing NAR hash of a large nixpkgs checkout (~300MB unpacked) takes seconds. Mitigation: cache the result; this only happens once per unique narHash.
- **[Temp directory disk usage]** → Large inputs (nixpkgs ~300MB) consume worker disk. Mitigation: clean up on cache drop; bounded by number of unique inputs in a flake.lock (typically < 20).
- **[ureq as new dep]** → Adds a dependency. Mitigation: ureq is small (~5 crates), well-maintained, and has no async runtime dependency. Alternative: use `curl` subprocess if dep is unwanted.
- **[Private repos]** → No auth token support. Mitigation: document limitation; private repo inputs fall back to subprocess which inherits system nix config (access-tokens).
