# Decision: Simplified Cargo Development Workflow

**Date**: 2026-01-14
**Status**: Implemented

## Context

The project documentation previously instructed developers to wrap every cargo command with `nix develop -c` or queue them through `pueue`:

```bash
# Old approach
nix develop -c cargo build
nix develop -c cargo nextest run
pueue add -- nix develop -c cargo build
```

This was verbose and obscured the actual development workflow.

## Decision

Simplify to native cargo inside the Nix development shell:

```bash
# New approach
nix develop        # Enter the shell once
cargo build        # ~2-3s incremental rebuilds
cargo nextest run  # Run tests
```

## Rationale

1. **Faster iteration**: The `nix develop` shell includes Mold linker, incremental compilation, and shared target directory - all configured automatically

2. **Simpler mental model**: Enter shell once, use native cargo commands

3. **Same environment guarantees**: The shell still provides reproducible tooling (rust-analyzer, cargo-nextest, cargo-llvm-cov, etc.)

4. **Correct pueue usage**: Reserve pueue for actually long-running tasks (Terraform deployments, NixOS updates), not cargo builds that complete in seconds

## Files Updated

- `.claude/CLAUDE.md` - Development Commands section
- `README.md` - Development section
- `AGENTS.md` - Build, Test, and Development Commands section
- `~/.claude/CLAUDE.md` - Rust-Specific Preferences and Command Discovery sections

## Quick Reference

```bash
nix develop              # Enter shell (once)
cargo build              # Build (~2-3s incremental)
cargo run --bin aspen-node -- --node-id 1 --cookie my-cluster
cargo run --bin aspen-cli -- kv get mykey
cargo nextest run        # All tests
cargo nextest run -P quick  # Quick tests (~2-5 min vs ~20-30 min)
cargo clippy --all-targets -- --deny warnings
cargo bench
nix fmt                  # Format (works inside or outside shell)
```
