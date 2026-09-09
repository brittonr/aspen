## Why

F03 identifies a lost-ingress window at revision `fa1ced3e808861d8ce59f02a6fd6b13b655f5147`.

An admitted envelope commits its delivery identity before inbox publication. A crash or write error after that commit leaves no inbox request. An exact retry reaches duplicate suppression. The duplicate path converts missing queue evidence to `None` through `.ok()` and returns no diagnostic. The enclosing path can report success without enqueue.

Expected behavior reconciles durable publication state and preserves uncertainty. It must also handle a crash after inbox publication but before receipt publication. Missing receipt evidence alone cannot authorize another enqueue or dispatch.

This finding has static transaction and error-path evidence only. No crash injection ran. The audit reports eight executed bugs and six static findings. F03 belongs to the static group.

## What Changes

- Define recoverable ingress publication states across dedup, inbox, dispatch, and receipt evidence.
- Reconcile exact operation identity before recovery effects.
- Distinguish known absence, existing publication, completed dispatch, and uncertain observation.
- Reject silent success and blind replay after ambiguous storage outcomes.
- Add normal repository fault-boundary tests with controlled adapters.

## Impact

- Current consumer: node-control ingress delivery and its retry path.
- Maintenance owner: Molten node-runtime maintainers with delivery-idempotency and node-host maintainers.
- Source scope: `src/node/parts/daemon/p026/body.rs`, `p018/body.rs`, and delivery idempotency persistence.
- Spec delta: package-local `node-runtime` requirements.
- Durable capability: explicit local ingress reconciliation and repeatable crash-window regressions.
- Repeatability evidence: planned reopen tests at each publication boundary, including receipt-loss cases.

## Non-goals and authority

This plan grants no implementation permission. It does not promise exactly-once external effects, storage durability after ambiguous errors, consensus recovery, or whole-system availability. `recover-from-storage-faults` owns general storage quarantine and recovery policy. This package consumes those decisions without expanding into Raft or peer repair.
