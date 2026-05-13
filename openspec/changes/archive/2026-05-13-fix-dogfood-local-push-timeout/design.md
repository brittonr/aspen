# Design

## Evidence source

The parent sweep's committed evidence is `openspec/changes/prove-gated-runtime-confidence/evidence/dogfood-full.md`. The datapool run reached cluster start, repo creation, and CI watch registration before timing out at `push` after roughly 602 seconds.

## Approach

1. Split the dogfood push stage into named sub-boundaries in logs and receipts: local git invocation started, forge receive-pack connected, pack/object ingest started, hook dispatch started, CI trigger accepted, and push completed.
2. Ensure each sub-boundary records elapsed duration and a redacted failure category in the dogfood stage receipt.
3. Add a focused local check that creates a dogfood cluster/repo and performs the minimal push needed to assert CI trigger acceptance. The check should be cheaper than a full build/deploy/verify run.
4. Use bounded timeouts around each push sub-boundary so failures become deterministic receipt classifications instead of a long opaque timeout.

## Verification

- Focused dogfood push/CI-trigger rail passes or records a classified failure boundary.
- `nix run .#dogfood-local -- --cluster-dir <large-local-path> full` reaches build/deploy/verify acceptance, or the new receipt identifies the exact remaining product failure boundary.
- `openspec validate fix-dogfood-local-push-timeout --strict --json` passes.
