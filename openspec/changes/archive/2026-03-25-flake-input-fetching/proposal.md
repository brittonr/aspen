## Why

The in-process flake evaluator (`evaluate_flake_derivation`) falls back to a `nix eval --raw .drvPath` subprocess whenever flake inputs aren't already present in the local `/nix/store`. The stub fetchers (`fetch_github_input`, `fetch_tarball_input`) return `NotFound`, so any CI worker that hasn't previously built the same flake.lock revision hits the subprocess path. Implementing HTTP fetching for GitHub and tarball inputs eliminates this subprocess dependency and moves closer to a fully self-contained CI executor.

## What Changes

- Replace the stub `fetch_github_input()` with an HTTP fetcher that downloads the GitHub archive tarball, unpacks it to a temp directory, and returns the unpacked path for use as the `outPath` override in call-flake.nix evaluation.
- Replace the stub `fetch_tarball_input()` with an HTTP fetcher that downloads and unpacks arbitrary tarball URLs.
- Add `fetch_git_input()` for git-type inputs (clone at pinned rev).
- Verify fetched content against the `narHash` from flake.lock before accepting it.
- Add a fetch cache to avoid re-downloading inputs that were already fetched in the same worker lifetime.
- Remove the `is_local` early-exit in `evaluate_flake_derivation` — fetched inputs are usable the same as local store paths.

## Capabilities

### New Capabilities

- `flake-input-fetch`: HTTP fetching of flake inputs (GitHub archives, tarballs) with narHash verification and caching.

### Modified Capabilities

- `snix-eval-integration`: The "Evaluator resolves flake input" scenario currently relies on snix-glue fetchers. This change implements fetching at the flake_lock layer instead, resolving inputs before evaluation begins.

## Impact

- **Code**: `crates/aspen-ci-executor-nix/src/flake_lock.rs` (fetcher implementations), new `fetch.rs` module for HTTP + unpack logic.
- **Dependencies**: `reqwest` (HTTP client, already a workspace dep), `flate2`/`tar` for tarball extraction (check workspace deps).
- **Tests**: Integration tests that fetch real GitHub archives (network profile), unit tests with mock HTTP responses.
- **Feature flags**: Fetching gated behind existing `snix-build` feature (only needed when in-process eval is active).
