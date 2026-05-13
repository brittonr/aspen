# Fix dogfood local push timeout

## Summary

Follow up from `prove-gated-runtime-confidence`: `nix run .#dogfood-local -- --cluster-dir /home/brittonr/data/aspen-dogfood-proof full` reached cluster start, Forge repository creation, CI watch registration, and the start of `git push`, then timed out at the local hook/push boundary before build/deploy/verify acceptance. Aspen needs a scoped repair that makes the local dogfood push boundary deterministic and diagnosable enough to complete or fail quickly with actionable evidence.

## Problem

The gated runtime confidence sweep classified this as the only real product follow-up after lower runtime tiers and `nix flake check -L --max-jobs 1` passed. The dogfood run did not reach self-hosted build/deploy/verify acceptance because the push stage exceeded its timeout. Without a focused change, future dogfood proofs can spend minutes waiting and still leave operators without a narrow cause: forge receive-pack, hook execution, CI trigger, federation/watch publication, or local transport readiness.

## Scope

- Bound and instrument the local dogfood `git push` stage.
- Preserve redacted receipt/log evidence for the first failing push sub-boundary.
- Add the smallest deterministic check or harness rail that reproduces local dogfood push acceptance without requiring the entire `full` pipeline.
- Re-run `dogfood-local -- full` (or classify a smaller proven boundary if host constraints remain) after the focused repair.

## Non-goals

- Redesign Forge storage or CI orchestration broadly.
- Replace the dogfood pipeline with a mock.
- Require relay/mDNS for local same-host dogfood connectivity.
