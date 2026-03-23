## Why

Aspen's zero-subprocess CI pipeline cannot evaluate flakes with `type = "git"` inputs. snix-eval's `builtins.fetchGit` returns `ErrorKind::NotImplemented("fetchGit")`, and the `flake_lock.rs` resolver explicitly returns `Unsupported` for `"git"` input types. This forces a fallback to `nix eval`/`nix build` subprocesses, breaking the self-hosted build goal. Aspen's own `flake.lock` has a `type=git` input (`spectrum` from `https://spectrum-os.org/git/spectrum`), so Aspen cannot build itself through its own zero-subprocess pipeline.

## What Changes

- Implement `builtins.fetchGit` in snix-glue's fetcher builtins, replacing the `NotImplemented` stub. The implementation uses `git` CLI subprocess (always available in Nix environments) to clone/fetch repos, then ingests the checkout into snix's castore. Returns an attrset with `outPath`, `rev`, `shortRev`, `lastModified`, `revCount`, `narHash`, `submodules` — matching Nix's behavior.
- Fill in the `Fetch::Git` variant in snix-glue's `Fetch` enum with proper fields (`url`, `rev`, `ref`, `shallow`, `submodules`, `allRefs`, `narHash`), replacing the empty `Git()` stub and its `todo!()` calls.
- Add `fetch_git_input()` to `flake_lock.rs` in `aspen-ci-executor-nix`, following the existing `fetch_github_input()`/`fetch_tarball_input()` pattern. Uses `git clone --bare` + `git archive` with narHash verification via the existing `FetchCache`.
- Add `fetch_and_clone_git()` to `fetch.rs` alongside the existing `fetch_and_unpack_tarball()`, implementing git clone, sparse checkout, and archive extraction with Tiger Style resource bounds.

## Capabilities

### New Capabilities

- `fetchgit-builtin`: Implementation of `builtins.fetchGit` in snix-glue that clones git repos, ingests checkouts into castore, and returns the attrset Nix expects. Covers argument parsing, caching, ref resolution, submodule support, and narHash verification.
- `fetchgit-flake-input`: Resolution of `type = "git"` locked inputs in flake.lock via git CLI, with FetchCache integration and narHash verification.

### Modified Capabilities

## Impact

- **snix-glue vendored/patched code**: `builtins/fetchers.rs` (new builtin body), `fetchers/mod.rs` (Fetch::Git variant filled in). These files live in snix's crate — changes need to be managed as a patch or fork.
- **aspen-ci-executor-nix**: `flake_lock.rs` (new `fetch_git_input` function wired into resolver), `fetch.rs` (new `fetch_and_clone_git` function).
- **Dependencies**: No new crate dependencies — uses `git` CLI via `std::process::Command` (same pattern as `curl` in `fetch.rs`).
- **Test coverage**: NixOS VM tests (`snix-flake-native-build-test`, new git-input test), unit tests for argument parsing and cache behavior.
- **Dogfood pipeline**: Unblocks `ci-dogfood-self-build` from needing subprocess fallback for git inputs.
