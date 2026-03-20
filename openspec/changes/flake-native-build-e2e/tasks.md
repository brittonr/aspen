## 1. Embed flake-compat

- [x] 1.1 Fetch NixOS/flake-compat `default.nix` at a pinned commit, add as `const FLAKE_COMPAT_NIX: &str` in a new `flake_compat.rs` module
- [x] 1.2 Add helper `write_flake_compat_to_temp() -> io::Result<PathBuf>` that writes the constant to a temp file and returns the path
- [x] 1.3 Unit test: verify embedded flake-compat parses without syntax errors via `rnix::Root::parse`

## 2. Flake-compat eval method

- [x] 2.1 Add `evaluate_flake_via_compat(&self, flake_dir: &str, attribute: &str) -> Result<(StorePath, Derivation), NixEvalError>` to `NixEvaluator` — writes flake-compat to temp, constructs `(import <path> { src = <flake_dir>; }).outputs.<attr>.drvPath`, evaluates with store, extracts Derivation from KnownPaths
- [x] 2.2 Unit test: evaluate a simple flake (`inputs = {}`, trivial derivation) via flake-compat — verify Derivation is returned with correct system/builder/outputs
- [x] 2.3 Unit test: evaluate a flake with a path input (local directory) via flake-compat — verify input resolution works through `builtins.path`

## 3. Wire into build pipeline

- [x] 3.1 Update `try_flake_eval_native` in `executor.rs` to call `evaluate_flake_via_compat` instead of the current `evaluate_flake_derivation` — keep the current method as fallback on error
- [x] 3.2 Add log line "flake native build completed (zero subprocesses)" when the full flake-compat → native build path succeeds
- [x] 3.3 Gate the old `evaluate_flake_derivation` call path behind `#[cfg(feature = "legacy-flake-eval")]` — enabled by default initially

## 4. VM Integration Test

- [x] 4.1 Create `nix/tests/snix-flake-native-build.nix` — boots aspen-node, starts `python3 -m http.server` serving a synthetic `.tar.gz`, creates a flake.nix with a `tarball` input, generates flake.lock with pre-computed narHash and URL
- [x] 4.2 VM test: submit ci_nix_build job, wait for completion, assert success
- [x] 4.3 VM test: assert journal contains "zero subprocesses" and does NOT contain "nix eval --raw"
- [x] 4.4 VM test: start cache gateway, verify it serves narinfo for the build output
- [x] 4.5 Wire test into flake.nix checks as `snix-flake-native-build-test`

## 5. Documentation

- [x] 5.1 Update `docs/nix-integration.md` — replace "eval subprocess bridge" description for path #2 with "flake-compat via snix-eval (zero subprocesses for github/gitlab/tarball/path inputs)"
- [x] 5.2 Add note about `fetchGit` limitation: git inputs fall back to subprocess until snix implements fetchGit
