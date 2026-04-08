## 1. Diagnose the exact CI clippy failure

- [x] 1.1 Add verbose logging to `NixBuildWorker::execute_build` — log the working directory path, flake ref, and full command before executing nix build
- [x] 1.2 Run `dogfood-local -- start` then `dogfood-local -- push` and manually inspect the Forge checkout directory structure (verify flake.nix, flake.lock, .git presence)
- [x] 1.3 Manually run `nix build .#checks.x86_64-linux.clippy` inside the Forge checkout directory to reproduce the exact error

## 2. Fix Forge checkout flake compatibility

- [x] 2.1 Verify `git-remote-aspen` includes `flake.lock` in the push (check `.gitignore` patterns and forge push filtering)
- [x] 2.2 In the CI executor's working directory setup, initialize the checkout as a git repo (`git init && git add -A && git commit -m "ci"`) before running nix build
- [x] 2.3 Ensure the CI executor sets `--option sandbox false` or configures nix to resolve flake inputs from the local store (no network in CI sandbox)
- [x] 2.4 Test that `nix build .#checks.x86_64-linux.clippy` succeeds on the prepared Forge checkout directory

## 3. Fix CI job log capture on failure

- [x] 3.1 In `NixBuildWorker::execute_build`, after subprocess exits with non-zero code, drain remaining stderr lines with a 1-second timeout before dropping the log sender
- [x] 3.2 Include the last 50 lines of stderr in the `JobResult::failure` message as a fallback when log streaming may have missed output
- [x] 3.3 Write a test that verifies stderr from a fast-failing nix build is captured in the KV log store

## 4. Improve dogfood failure reporting

- [x] 4.1 In `fetch_failure_logs`, when `CiGetJobLogs` returns empty chunks, fall back to the job's `result_message` field from `CiGetStatus`
- [x] 4.2 Print the failure detail with a clear header: `--- Failed job: <name> ---\n<log output>`
- [x] 4.3 Increase `CiGetJobLogs` limit from 50 to 200 chunks to capture more context from long builds

## 5. End-to-end verification

- [x] 5.1 Run `nix run .#dogfood-local -- full` and verify the clippy job passes
- [x] 5.2 Introduce a deliberate clippy warning and verify the dogfood pipeline reports the actual warning text in its failure output
- [x] 5.3 Remove the deliberate warning and verify the full pipeline completes (clippy → build → test → deploy → verify)
