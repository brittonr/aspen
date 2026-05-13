## Context

`push-check` reaches repo creation and CI watch registration, then times out in `git push aspen-dogfood`. The local Forge repo is empty, so pushing `HEAD` from the developer checkout requires transferring the full reachable history. For dogfood-local acceptance, the relevant product boundary is current-source ingestion plus CI trigger, not preserving every historical commit.

## Goals / Non-Goals

**Goals:**
- Push the current source content through the real `git-remote-aspen` helper and Forge RPC path.
- Avoid full-history transfer in the bounded local acceptance rail.
- Keep receipt classification redacted and deterministic.

**Non-Goals:**
- Replacing normal developer pushes or federation sync semantics.
- Claiming full-history migration performance.

## Decisions

### 1. Snapshot push workspace

**Choice:** Before the dogfood-local push, materialize `HEAD` with `git archive`, initialize a temporary Git repo under the cluster directory, create one local commit, and push that commit to Aspen Forge.

**Rationale:** This keeps the content under test identical to the committed source tree while reducing the object graph from all ancestors to one commit/tree/blob set. It exercises the same remote helper, Forge object ingestion, ref update, and CI trigger path.

**Alternative:** Increase the timeout. Rejected because it preserves the wrong acceptance bottleneck and still obscures product regressions.

## Risks / Trade-offs

**Snapshot omits untracked files** → This is intentional; dogfood should prove the committed source.

**Snapshot commit differs from original commit hash** → Receipt already records the source checkout commit; the pushed commit may carry a provenance message.
