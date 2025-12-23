# Test Caching Analysis for Aspen

**Created: 2025-12-19**

## Executive Summary

After comprehensive research across cargo-mutants, cargo-llvm-cov, cargo-nextest, and analysis of Aspen's test infrastructure, the key finding is:

**Neither cargo-mutants nor cargo-llvm-cov directly solve "test caching" - they serve different purposes entirely.**

The actual problem (skipping unchanged tests) has **no mature Rust-native solution** as of December 2025. The cargo team has an open issue ([#10673](https://github.com/rust-lang/cargo/issues/10673)) but it's blocked.

## Tool Purpose Comparison

| Tool | Purpose | Does it cache test results? |
|------|---------|----------------------------|
| **cargo-mutants** | Mutation testing (finds weak tests) | No - runs ALL tests for each mutation |
| **cargo-llvm-cov** | Code coverage measurement | No - generates coverage data, doesn't cache |
| **cargo-nextest** | Fast test runner | No - caches binaries only, not results |
| **sccache** | Compilation caching | No - speeds up builds, not test runs |

## What Each Tool Actually Does

### cargo-mutants

- **Purpose**: Find places where bugs can be inserted without tests failing
- **How it works**: Modifies code (mutations), runs full test suite per mutation
- **Useful for**: Finding inadequate test coverage
- **Relevant features**:
  - `--in-diff` - Only mutate code changed in a diff (for PRs)
  - `--iterate` - Skip previously caught mutants
  - `--baseline=skip` - Skip baseline test run (assumes tests pass)
- **NOT useful for**: Skipping tests for unchanged code

### cargo-llvm-cov

- **Purpose**: Measure which lines of code are executed by tests
- **How it works**: Instruments code, runs tests, generates coverage reports
- **Useful for**: Finding untested code paths
- **Relevant features**:
  - Per-test coverage data (JSON export)
  - Integration with cargo-nextest
  - LCOV output for CI tools
- **Could theoretically enable**: Test Impact Analysis (TIA) with custom tooling
- **NOT useful for**: Automatic test skipping (requires custom implementation)

## Aspen-Specific Constraints

Based on codebase analysis:

| Constraint | Impact on Caching |
|------------|-------------------|
| **Single-threaded tests** (`test-threads = 1`) | Cannot parallelize to speed up |
| **Madsim simulation** | Each seed produces different results - cannot cache |
| **350+ tests, ~20-30 min full run** | Significant incentive to optimize |
| **Deterministic design** | Good for reproducibility, bad for caching |
| **Property-based tests** | Run multiple iterations, timing varies |

## Available Strategies (Ranked by Practicality)

### 1. Build Caching (Already Implemented)

**Status**: You already have this via Nix/Cachix

What's working:

- `cargoArtifacts = craneLib.buildDepsOnly` in flake.nix
- Cachix for CI binary caching
- Incremental compilation

**Improvement**: Ensure sccache is enabled in development:

```bash
export RUSTC_WRAPPER=sccache
```

### 2. Nextest Archiving (For CI)

**Status**: Available, not currently used

```bash
# On build machine
cargo nextest archive --archive-file tests.tar.zst

# On test machine (skips rebuild entirely)
cargo nextest run --archive-file tests.tar.zst
```

**Use case**: Ship pre-built test binaries to CI runners

### 3. Nextest Filtersets (Manual Selection)

**Status**: Available now

```bash
# Skip slow tests during development
cargo nextest run -E 'not test(/proptest/) and not test(/chaos/)'

# Run only tests for a specific module
cargo nextest run -E 'test(/raft::/) or test(/storage/)'
```

**Add to nextest.toml**:

```toml
[profile.quick]
test-threads = 1
default-filter = "not test(/proptest/) and not test(/chaos/) and not test(/madsim/)"
slow-timeout = { period = "30s", terminate-after = 1 }
```

### 4. cargo-mutants `--in-diff` (For PRs)

**Status**: Excellent for CI, different purpose

```yaml
# .github/workflows/mutants.yml
- name: Mutate changed code only
  run: |
    git diff origin/${{ github.base_ref }}.. > diff.patch
    cargo mutants --in-diff diff.patch --baseline=skip
```

**What this does**: Tests mutation coverage ONLY for changed code

### 5. Custom Test Impact Analysis (TIA)

**Status**: Requires implementation, high effort

The theoretical approach:

1. Run tests once with `cargo-llvm-cov` to generate per-test coverage maps
2. On file change, determine which tests cover that file
3. Run only affected tests

**Implementation sketch**:

```bash
# Generate per-test coverage (expensive, run weekly)
cargo llvm-cov --json --output-path coverage-map.json

# On change: parse coverage-map.json, find tests that touch changed files
# This requires custom tooling - no off-the-shelf solution exists
```

**Reality check**: This is how Facebook/Google do it, but they have dedicated teams.

## Recommended Approach for Aspen

Given your constraints (madsim, determinism, 350+ tests):

### For Local Development

1. **Use filtersets** for quick iteration:

   ```bash
   # Add alias to shell config
   alias qt='nix develop -c cargo nextest run -E "not test(/proptest/) and not test(/chaos/) and not test(/madsim/)"'
   ```

2. **Create a "quick" profile** in `.config/nextest.toml`:

   ```toml
   [profile.quick]
   test-threads = 1
   default-filter = "not test(/proptest/) and not test(/chaos/) and not test(/madsim_multi/)"
   slow-timeout = { period = "30s", terminate-after = 1 }
   ```

   Usage: `cargo nextest run -P quick`

3. **Run targeted tests**:

   ```bash
   # Only tests matching your current work
   cargo nextest run -E 'test(/storage/)'
   cargo nextest run -E 'test(/gossip/)'
   ```

### For CI

1. **Add nextest archiving** for cross-job artifact reuse:

   ```yaml
   - name: Build test archive
     run: cargo nextest archive --archive-file tests.tar.zst

   - name: Upload archive
     uses: actions/upload-artifact@v4
     with:
       name: test-archive
       path: tests.tar.zst
   ```

2. **Add cargo-mutants for PR reviews**:

   ```yaml
   incremental-mutants:
     if: github.event_name == 'pull_request'
     steps:
       - run: git diff origin/${{ github.base_ref }}.. > diff.patch
       - run: cargo mutants --in-diff diff.patch --baseline=skip --timeout 300
   ```

### NOT Recommended

- **Building custom TIA**: Too much effort for a 350-test suite
- **Waiting for cargo native caching**: No timeline, blocked indefinitely
- **Using cargo-llvm-cov for test selection**: No tooling exists, would need custom dev

## Comparison: What Other Projects Do

| Project | Approach | Notes |
|---------|----------|-------|
| **rustc** | Custom compiletest + caching | Massive investment, not portable |
| **RisingWave** | Madsim + sharding | Similar to Aspen |
| **TigerBeetle** | VOPR + deterministic replay | Fixed seeds, not cached |
| **FoundationDB** | Custom simulation framework | Years of development |

## Conclusion

The honest answer is: **There is no mature "test caching" solution for Rust.**

The practical path forward for Aspen:

1. **Short-term**: Use nextest filtersets and profiles (`cargo nextest run -P quick`)
2. **Medium-term**: Add nextest archiving to CI for build/test separation
3. **For mutation testing**: Add `cargo-mutants --in-diff` to PR workflow (different goal, but useful)
4. **Skip**: Custom TIA, waiting for cargo native caching

Your best ROI is the `quick` profile that skips long-running property/chaos/madsim tests during development.

## Sources

- [Cargo Test Caching Issue #10673](https://github.com/rust-lang/cargo/issues/10673)
- [cargo-nextest Running Tests](https://nexte.st/book/running.html)
- [cargo-nextest Archiving](https://nexte.st/docs/ci-features/archiving/)
- [cargo-mutants PR Diff Testing](https://mutants.rs/pr-diff.html)
- [cargo-mutants Performance](https://mutants.rs/performance.html)
- [cargo-llvm-cov GitHub](https://github.com/taiki-e/cargo-llvm-cov)
- [Test Impact Analysis - Martin Fowler](https://martinfowler.com/articles/rise-test-impact-analysis.html)
- [rust-analyzer Live Testing Issue #8420](https://github.com/rust-lang/rust-analyzer/issues/8420)
