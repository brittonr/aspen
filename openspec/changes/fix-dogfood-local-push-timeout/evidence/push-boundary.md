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
