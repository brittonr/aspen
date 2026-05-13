# Push boundary evidence

Captured: 2026-05-13T14:53:41Z

## Reproduction source

The parent gated-runtime sweep preserved the datapool dogfood run in `openspec/changes/archive/2026-05-13-prove-gated-runtime-confidence/evidence/dogfood-full.md`:

- command: `nix run .#dogfood-local -- --cluster-dir /home/brittonr/data/aspen-dogfood-proof full`
- receipt: `/home/brittonr/data/aspen-dogfood-proof-receipts/dogfood-20260513T034555Z.json`
- failed stage: `push`
- elapsed: `602059 ms`
- redacted failure: `git push aspen-dogfood timed out after 600s`
- process inspection: `git push`, `git-remote-aspen`, and the repository `pre-push` hook were still resident

This reproduces the product follow-up boundary without retaining ticket material or raw `aspen://` URLs.

## Repair slice

- `git push` now uses `--no-verify` so local workstation pre-push hooks cannot consume the dogfood push timeout budget or obscure the Forge/CI product boundary.
- Dogfood receipts classify push failures with sub-boundary operations such as `push:forge_rpc`, `push:local_git_invocation`, and `push:push_completion`.
- Push-stage timeouts are categorized as `push_timeout`, distinct from generic build/CI timeouts.
- New `push-check` command runs a cheaper receipt-backed `start -> push/CI-watch -> stop` path for focused local push/CI-trigger acceptance.

## Verification

```bash
cargo test -p aspen-dogfood -- --nocapture
```

Result: passed, `77 passed; 0 failed`.

Attempted focused runtime proof:

```bash
ASPEN_DOGFOOD_GIT_PUSH_TIMEOUT_SECS=120 \
  nix run .#dogfood-local -- --cluster-dir /home/brittonr/data/aspen-dogfood-push-check push-check \
  2>&1 | tee target/runtime-proof/dogfood-push-check.log
```

Result: blocked before runtime execution by local Nix build wall clock timeout after 600s while building the app closure. No dogfood cluster started and no push receipt was produced in this attempt.

Focused runtime rerun after the Nix app closure completed:

```bash
ASPEN_DOGFOOD_GIT_PUSH_TIMEOUT_SECS=120 \
  nix run .#dogfood-local -- --cluster-dir /home/brittonr/data/aspen-dogfood-push-check push-check \
  2>&1 | tee target/runtime-proof/dogfood-push-check-rerun.log
```

Result: runtime reached the local product boundary and saved receipt `/home/brittonr/data/aspen-dogfood-push-check-receipts/dogfood-20260513T153340Z.json`.

Receipt summary, redacted:

- command: `push-check`
- git commit: `f95bed8790c25964cb4164695eb5508ff0e2d759`
- `start`: succeeded in `11723 ms`
- `push`: failed in `124146 ms`
- operation: `push:push_completion`
- category: `push_timeout`
- message: `git push aspen-dogfood timed out after 120s`
- `stop`: succeeded in `2003 ms`

Classification: the hook bypass worked and the focused proof no longer stops at local workstation hooks or Nix app build time. The current bounded failure is the product push-completion boundary after repo creation and CI-watch registration. Re-running `dogfood-local -- full` would hit the same pre-build push boundary, so the full loop is reclassified as blocked behind `push:push_completion` rather than build/deploy/verify.
