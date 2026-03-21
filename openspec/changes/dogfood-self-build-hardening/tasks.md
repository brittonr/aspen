## 1. Run dogfood-local.sh and fix failures

- [x] 1.1 Run `nix run .#dogfood-local -- start` and verify single-node cluster starts with CI features enabled
- [x] 1.2 Run `nix run .#dogfood-local -- push` and verify full workspace pushes to Forge via git-remote-aspen
- [x] 1.3 Run `nix run .#dogfood-local -- build` and wait for CI pipeline; diagnose and fix any pipeline failures (check stage: format-check, clippy; build stage: build-node, build-cli; test stage: nextest-quick)
- [x] 1.4 Run `nix run .#dogfood-local -- deploy` and verify CI-built binary replaces the running node
- [x] 1.5 Run `nix run .#dogfood-local -- verify` and confirm CI-built binary matches expectations (version string, size, functionality)
- [x] 1.6 Run `nix run .#dogfood-local -- full-loop` end-to-end to confirm the complete cycle works unattended

## 2. Harden dogfood-local.sh error handling

- [x] 2.1 Fix stale cluster state handling: clean up leftover PID files and sockets from previous runs in `do_start`
- [x] 2.2 Improve CI failure reporting: print failing stage/job name and tail of log output when pipeline status is "failed"
- [x] 2.3 Fix `stream_pipeline` cleanup: ensure background log-tail processes are killed on all exit paths (not just trap EXIT)
- [x] 2.4 Handle missing `nix` binary gracefully in deploy: validate CI-built binary exists before attempting stop/restart
- [x] 2.5 Add timeout to `do_push` for large workspace pushes (git push can hang on broken iroh connections)

## 3. NixOS VM integration test

- [x] 3.1 Create `nix/tests/ci-dogfood-full-loop.nix` VM test that boots a single node with CI + Forge + snix features
- [x] 3.2 Pre-populate VM nix store with crane cargo artifacts and ciSrc so inner `nix build` doesn't need network
- [x] 3.3 Push workspace source to Forge, enable CI watch, and wait for auto-triggered pipeline
- [x] 3.4 Verify CI pipeline completes all 3 stages (check, build, test) with status "success"
- [x] 3.5 Extract output path from build-and-test job result and run the CI-built `aspen-constants-check --version`
- [x] 3.6 Wire test into flake.nix as `checks.x86_64-linux.ci-dogfood-full-loop-test`

## 4. Validation

- [x] 4.1 Run `nix run .#dogfood-local -- full-loop` from clean state and confirm it passes
- [x] 4.2 Build the VM test: `nix build .#checks.x86_64-linux.ci-dogfood-full-loop-test --option sandbox false`
- [ ] 4.3 Run existing dogfood tests to confirm no regressions: `ci-dogfood-test`, `ci-dogfood-self-build-test`
