## Context

The CI executor has three build paths, tried in priority order: (1) zero-subprocess npins, (2) flake native with `nix eval` subprocess, (3) full `nix build` subprocess. Path 2 still shells out to `nix eval --raw .#attr.drvPath` to resolve the derivation. All other pieces — the bwrap sandbox, output upload, PathInfoService ingestion — are already in-process via snix-build.

Nix internally evaluates flakes through a clean protocol: parse `flake.lock`, fetch/resolve each locked input to a store path, then evaluate `call-flake.nix` (a 90-line pure Nix expression) with three arguments: the lock file JSON, an overrides attrset of pre-fetched inputs, and a `fetchTreeFinal` builtin for any remaining fetches. The `call-flake.nix` expression handles follows resolution, input graph construction, and `flake.outputs` invocation.

snix-eval at rev `e20f82dd` supports: `derivationStrict`, `import`, `fetchTarball`, `parseFlakeRef`, store-backed I/O via `SnixStoreIO`. It lacks: `getFlake`, `fetchTree`, `fetchGit`, and flake lock resolution. But these gaps are bridgeable — we don't need full flake support in snix-eval, we just need to pre-resolve inputs in Rust and pass them as overrides.

## Goals / Non-Goals

**Goals:**

- Eliminate `nix eval` subprocess from the flake native build path
- Reuse Nix's `call-flake.nix` expression verbatim for correctness
- Compute input store paths from `narHash` in `flake.lock` via nix-compat `build_ca_path` (no fetching when inputs already exist locally)
- Fetch missing inputs using snix's `fetchTarball` (github/gitlab/tarball types) or filesystem copy (path type)
- Fall back to `nix eval` subprocess when in-process eval fails (IFD, unsupported builtins, git-type inputs without local store path)

**Non-Goals:**

- Implementing `builtins.getFlake` in snix-eval (we bypass it via call-flake.nix)
- Implementing `fetchGit` in snix (git-type inputs without a local store path fall back to subprocess)
- Supporting unlocked flake inputs (CI always has flake.lock)
- Replacing the `nix build` subprocess fallback path (path 3 stays as-is)

## Decisions

### D1: Use Nix's call-flake.nix directly

Embed the 90-line `call-flake.nix` from upstream Nix (libflake) as a string constant. Evaluate it via snix-eval with pre-built arguments. This mirrors exactly what Nix does internally, guaranteeing semantic equivalence.

Alternative: Reimplement flake resolution in pure Rust. Rejected — duplicates complex logic (follows resolution, relative paths, `sourceInfo` construction, `self` wiring) and diverges from Nix semantics.

### D2: Pre-resolve inputs in Rust, pass as overrides

Parse `flake.lock` in Rust, compute store paths via `build_ca_path(name="source", CAHash::Nar(Sha256(digest)), refs=[], self_ref=false)`, and build the overrides attrset. The `fetchTreeFinal` argument becomes a no-op because all inputs are in overrides.

This matches the `callFlake` C++ function in `flake.cc` — it builds the same overrides structure: `{ key: { sourceInfo: { outPath, narHash, rev, shortRev, lastModified, ... }, dir } }`.

For inputs not in the local store, fetch before building overrides: `fetchTarball` for github/gitlab/tarball types (snix has this), filesystem copy for path types. Git-type inputs without a local store path trigger fallback.

Alternative: Implement `fetchTreeFinal` as a snix-eval builtin that fetches on demand. Rejected — more complex, mixes Rust fetching with eval-time execution, and gains nothing since we can pre-fetch all locked inputs before eval starts.

### D3: Flake lock parser as a standalone module

New `flake_lock.rs` module with:

- `FlakeLock` struct: `{ version, root, nodes: HashMap<String, FlakeNode> }`
- `FlakeNode` struct: `{ inputs, locked: LockedInput, flake: bool }`
- `LockedInput` enum: `Github { owner, repo, rev, narHash }`, `Tarball { url, narHash }`, `Path { path }`, `Git { url, rev, narHash }`, `Indirect { ... }`
- `resolve_input_store_paths()`: computes `StorePath` for each node from `narHash` via `build_ca_path`
- `build_overrides_nix_expr()`: generates a Nix expression that constructs the overrides attrset from resolved store paths

Alternative: Generate the overrides in Rust as snix-eval `Value` objects directly. Rejected — `Value` construction requires `EvalState` access, and building attrsets programmatically is fragile. Generating a Nix string like `{ "nixpkgs" = { sourceInfo = { outPath = "/nix/store/..."; ... }; dir = ""; }; }` is simpler and lets snix-eval handle all the Value construction.

### D4: Construct a wrapper expression for evaluation

Generate a Nix expression that:

1. Defines the call-flake function inline
2. Passes the lock file JSON string
3. Passes the overrides attrset (with concrete store paths)
4. Passes `builtins.fetchTree` as `fetchTreeFinal` (available in snix-eval, used only as fallback for any input not in overrides)
5. Navigates to the requested attribute and forces `.drvPath`

```nix
let
  callFlake = <embedded call-flake.nix>;
  lockFileStr = ''<flake.lock JSON>'';
  overrides = {
    "root" = { sourceInfo = { outPath = "/path/to/checkout"; }; dir = ""; };
    "nixpkgs" = { sourceInfo = { outPath = "/nix/store/...-source"; narHash = "sha256-..."; rev = "abc..."; shortRev = "abc..."; lastModified = 1234; }; dir = ""; };
  };
  # fetchTreeFinal: no-op if everything is in overrides, but needed as argument
  fetchTreeFinal = attrs: throw "fetchTreeFinal called for ${attrs.type or "unknown"} — input not pre-resolved";
  flake = callFlake lockFileStr overrides fetchTreeFinal;
in flake.packages.x86_64-linux.default.drvPath
```

snix-eval evaluates this, triggers `derivationStrict` via the `.drvPath` access, and the `Derivation` object is extracted from `KnownPaths`.

### D5: Fallback on failure

If in-process flake eval fails (IFD, missing input, unsupported builtin), fall back to the existing `resolve_drv_path` subprocess (`nix eval --raw .drvPath`). Log a warning with the failure reason. This keeps the system working while we iterate on coverage.

## Risks / Trade-offs

**[Risk] snix-eval doesn't support all builtins used by flake.nix files** → Mitigation: fall back to subprocess. Log which builtin failed so we can track coverage gaps over time.

**[Risk] call-flake.nix may change between Nix versions** → Mitigation: pin to a specific version of call-flake.nix (from Nix 2.28.x, current stable). Add a comment with the source commit. The expression is stable — it's been essentially unchanged for 2+ years.

**[Risk] `fetchTreeFinal` may be called for inputs not in overrides (e.g., relative path inputs whose parent is in a different node)** → Mitigation: `call-flake.nix` handles relative paths by composing `parentNode.outPath + "/" + node.locked.path`. As long as the parent is in overrides, this works. We resolve all nodes, not just root inputs.

**[Risk] Large flake.lock files with deep input graphs** → Mitigation: bound resolution to `MAX_FLAKE_LOCK_NODES = 500` (Tiger Style). Real-world flakes rarely exceed 50 nodes.

**[Risk] narHash → store path computation mismatch with Nix** → Mitigation: use nix-compat's `build_ca_path` which is battle-tested against Nix's path computation. Add property-based tests comparing against `nix store path-from-hash-part`.
