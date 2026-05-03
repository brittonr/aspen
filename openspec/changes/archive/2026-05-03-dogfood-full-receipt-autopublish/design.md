## Context

Manual `receipts publish` proves Aspen can store dogfood evidence in Raft-backed KV, but it is still an extra operator step. The highest-ROI follow-up is to make the successful `full` path self-publish its final receipt while the cluster is known healthy.

## Goals / Non-Goals

**Goals:**

- Publish the canonical final success receipt automatically after `verify` succeeds.
- Keep publication before `stop`, while the state file and cluster ticket are still present.
- Add an explicit operator flag that skips automatic stop so `receipts cluster-show <run-id> --json` can be run after `full` exits.
- Preserve local receipt durability in all cases.

**Non-Goals:**

- Do not persist credential material or cluster tickets in receipts/docs.
- Do not change failure receipt semantics or publish partial failed receipts automatically.
- Do not introduce a new receipt schema version unless the receipt shape changes.

## Decisions

### Auto-publish is a post-verify/pre-stop action

**Choice:** After push/build/deploy/verify all succeed, write the current receipt, validate its run id, publish canonical JSON to `dogfood/receipts/<run-id>.json`, and then either stop or leave the cluster running based on CLI args.

**Rationale:** This is the point where the receipt contains all acceptance stages except cleanup and the cluster is still live and healthy. Publishing after `stop` cannot work because the cluster state/ticket has been removed.

### Leave-running mode is explicit

**Choice:** Add `aspen-dogfood full --leave-running` to skip automatic stop after successful verification and auto-publication.

**Rationale:** Normal `full` keeps cleanup behavior. Operators who want cluster-backed readback after command completion can opt into a visible, intentional running cluster.

## Risks / Trade-offs

**Publication failure after successful verify** → Treat it as a dogfood failure because self-hosted evidence publication is part of acceptance evidence. The local receipt remains available for diagnosis.

**Left-running clusters** → The flag is explicit, logs the cluster state, and docs tell operators to run `stop` when done.
