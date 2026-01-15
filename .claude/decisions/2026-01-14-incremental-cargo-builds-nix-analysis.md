# Incremental Cargo Builds in Nix: Analysis & Recommendations

**Date**: 2026-01-14
**Context**: Ultra mode analysis of incremental Cargo builds within Nix-based projects

## Executive Summary

Incremental Cargo builds in Nix represents a fundamental tension between:

- **Nix**: Hermetic, declarative builds where any input change invalidates derivations
- **Cargo**: Stateful incremental compilation with persistent `target/` artifacts

The solution is a **two-layer approach**:

1. **Development**: Use `nix develop` + native cargo (fastest iteration)
2. **CI/Packaging**: Use crane with `buildDepsOnly` + `cargoArtifacts` (cacheable, reproducible)

## Aspen's Current Setup (Excellent)

Aspen already implements best practices:

```nix
# flake.nix lines 171-217
cargoArtifacts = craneLib.buildDepsOnly basicArgs;

devCargoArtifacts = craneLib.buildDepsOnly (
  basicArgs // {
    doInstallCargoArtifacts = true;
    CARGO_INCREMENTAL = "1";
  }
);

devArgs = commonArgs // {
  cargoArtifacts = devCargoArtifacts;
  CARGO_INCREMENTAL = "1";
  CARGO_BUILD_INCREMENTAL = "true";
};
```

**Strengths**:

- Separate dev and production cargo artifacts
- Incremental compilation enabled in devShell
- Shared target directory (`CARGO_TARGET_DIR = "target"`)
- Modern resolver enabled (`CARGO_RESOLVER = "2"`)

## Solution Matrix

| Solution | Approach | Granularity | Best For |
| --- | --- | --- | --- |
| **crane** | Deps + source separation | Workspace-level | Production, CI (recommended) |
| **naersk** | Similar, simpler API | Workspace-level | Legacy, simple projects |
| **cargo2nix** | Per-crate derivations | Per-crate | Fine-grained caching |
| **nix develop + cargo** | Native cargo in Nix env | N/A | Development iteration |

## Crane Architecture

```
buildDepsOnly(basicArgs)
        |
        v
   cargoArtifacts (cached dependencies)
        |
        v
buildPackage(commonArgs // { inherit cargoArtifacts; })
```

Key functions:

- `buildDepsOnly`: Builds dependencies without source code (cache stable)
- `cargoArtifacts`: Pre-built dependency target directory
- `buildPackage`: Builds application using cached artifacts

## Development Workflow (Fastest)

```bash
nix develop                    # Enter Nix environment
cargo build                    # Native incremental (2-5s rebuilds)
cargo nextest run -P quick     # Fast tests
```

Aspen's devShell provides:

- Incremental compilation (`CARGO_INCREMENTAL=1`)
- Shared target directory
- lld linker (via Nix)
- All dev tools (rust-analyzer, cargo-nextest, etc.)

## CI/Production Workflow (Reproducible)

```bash
nix build                      # Uses crane + cargoArtifacts
nix flake check                # Runs all checks
```

Crane caches:

- Dependencies (rebuild only on Cargo.lock change)
- Artifacts reusable across clippy/test/build

## Optimization Opportunities

### 1. Mold Linker (20-30% faster incremental)

Currently using `lld`. Consider mold:

```nix
# In devShell
RUSTFLAGS = "-C link-arg=-fuse-ld=mold";
```

### 2. Source Filtering (Reduce unnecessary rebuilds)

Current: `filter = path: type: true;` (includes everything)

Better:

```nix
filter = path: type:
  (lib.hasSuffix ".rs" path) ||
  (lib.hasSuffix ".toml" path) ||
  (builtins.match ".*/(openraft|crates|vendor)/.*" path != null) ||
  type == "directory";
```

### 3. Private Binary Cache

TODO comments at lines 33-34, 38-39 indicate missing cache:

```nix
extra-substituters = [
  "https://cache.nixos.org"
  "https://your-harmonia-or-cachix.com"  # Add this
];
```

## Performance Comparison

| Scenario | Time | Notes |
| --- | --- | --- |
| Clean Nix build | 15-30 min | Full dependency compilation |
| Nix build (cached deps) | 2-5 min | Only source recompilation |
| `cargo build` (incremental) | 2-10s | Native Cargo, fastest |
| `cargo build` (clean) | 5-15 min | No cache |

## Key Takeaways

1. **Aspen is well-configured** - crane + devShell incremental already implemented
2. **Development uses native cargo** - Nix provides environment, cargo provides speed
3. **CI uses crane** - Reproducible, cacheable, shareable
4. **Mold linker** - Consider for additional speedup
5. **Binary cache** - Add private cache for team/CI sharing

## Sources

- [Crane: Composable Builds](https://ipetkov.dev/blog/introducing-crane/)
- [Crane GitHub](https://github.com/ipetkov/crane)
- [Crane API](https://crane.dev/API.html)
- [devenv 2025 Rust Integration](https://devenv.sh/blog/2025/08/22/closing-the-nix-gap-from-environments-to-packaged-applications-for-rust/)
- [Mold Linker](https://github.com/rui314/mold)
