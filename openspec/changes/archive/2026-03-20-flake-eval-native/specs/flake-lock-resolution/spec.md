## ADDED Requirements

### FLKLOCK-1: Parse flake.lock v7 format

The system MUST parse `flake.lock` JSON files conforming to version 7 (the current stable format). It MUST extract: `root` node name, per-node `inputs` map, per-node `locked` attributes (`type`, `narHash`, `rev`, `lastModified`, `owner`, `repo`, `url`, `path`), and per-node `flake` boolean (default true).

### FLKLOCK-2: Resolve follows

The system MUST resolve `follows` input specs. A follows spec is an array of strings representing a path from the root node through input names. The system MUST traverse the path and return the final node name. Circular follows MUST produce an error.

### FLKLOCK-3: Compute store paths from narHash

For each locked node with a `narHash` field (SRI format `sha256-<base64>`), the system MUST compute the expected Nix store path using `build_ca_path(name="source", CAHash::Nar(Sha256(digest)), refs=[], self_ref=false)` from nix-compat. The computed path MUST match what `nix flake metadata` produces.

### FLKLOCK-4: Detect input availability

For each computed store path, the system MUST check whether the path exists in the local `/nix/store`. Inputs that exist locally are marked as resolved. Inputs that don't exist are marked as needing fetch.

### FLKLOCK-5: Fetch missing github/gitlab inputs

For locked inputs of type `github` or `gitlab` that are not in the local store, the system MUST fetch the source tarball from the provider's API (`https://api.github.com/repos/{owner}/{repo}/tarball/{rev}` for github). The fetched tarball MUST be unpacked and stored. The resulting store path MUST match the narHash.

### FLKLOCK-6: Fetch missing tarball inputs

For locked inputs of type `tarball` with a `url` field, the system MUST download and unpack the tarball. The resulting store path MUST match the narHash.

### FLKLOCK-7: Handle path inputs

For locked inputs of type `path`, the system MUST resolve the path relative to the flake source directory. If the path is relative (doesn't start with `/`), it MUST be resolved relative to the parent input's outPath per call-flake.nix semantics.

### FLKLOCK-8: Git input fallback

For locked inputs of type `git` where the computed store path is not in the local store, the system MUST signal that in-process resolution is not possible (triggering fallback to `nix eval` subprocess).

### FLKLOCK-9: Build overrides attrset

The system MUST construct a Nix expression string representing the overrides attrset. Each node MUST have: `sourceInfo = { outPath = "<store-path>"; }` at minimum. When available from the locked entry, the sourceInfo MUST also include: `narHash`, `rev`, `shortRev` (first 7 chars of rev), `lastModified`.

### FLKLOCK-10: Resource bounds

The system MUST reject flake.lock files with more than 500 nodes. The system MUST reject narHash values that are not `sha256-` prefixed SRI hashes. The system MUST reject flake.lock versions other than 7.
