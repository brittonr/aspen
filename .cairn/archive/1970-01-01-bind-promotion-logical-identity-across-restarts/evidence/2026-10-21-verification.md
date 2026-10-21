# Promotion identity execution evidence — 2026-10-21

## Scope and ownership

Change: `bind-promotion-logical-identity-across-restarts`, on isolated worktree branch `cairn/next-audit-20261021`, based on `f79167c98`. Immediate outcome: a promotion retry is bound to one logical operation and one canonical request, so a same-operation retry with different parameters is denied instead of being re-planned, and identity survives restart. Durable contribution: the four identity roles in the core contract, a durable identity row written with the promotion, and typed replay decisions. Molten promotion-path maintainers own the core contract, store, adapter, and docs.

## Executed pre-change observation

The finding was a static source review. A temporary detached worktree at the base revision `f79167c98` ran the public promotion path with the repository's own store fixture: it committed one promotion, then retried the same logical operation with a different candidate head.

```text
pre-change retry=ok plan_ref_changed=true persistence=Conflicting dispatch_eligible=false
```

The retry was accepted as a new plan under a different plan ref and reported a conflicting persistence state. Nothing rejected the reuse of one logical operation with different request parameters, and no canonical request identity was bound to the operation. The temporary worktree and its probe test were removed after the observation.

## Implementation

- `WorldPromotionIdentity` carries the four roles: the logical `operation_ref`, a new `canonical_request_ref`, the `attempt_ref`, and the generation that authorizes the owner. `promotion_identity` derives the canonical request ref with a domain-separated BLAKE3 encoding of the exact request parameters (branch, class, both heads, expected generation, policy, authority ref and observed generation, closure and simulation flags, and the sorted intent closure) and derives the initial attempt ref from the operation and request identities.
- `evaluate_promotion_retry` is the pure decision: an unknown operation is new; the same operation with the same canonical request is a replay; the same operation with a different canonical request is `IncompatibleRequest`; a different operation is always new, so equal payloads never merge.
- `plan_world_promotion` attaches the identity to the plan, so the logical identity and the canonical request binding exist before any effect.
- The store writes a `world-promotion-identity` row in the same Redb transaction as the successor head, the promotion record, and the reservations, and refuses a commit whose stored identity differs from the planned identity. Reads are unknown-tolerant: a promotion record without an identity row is reported as a legacy logical operation.
- The service evaluates the stored identity before committing. A replay returns the retained outcome from read-back, a legacy record replays with retained uncertainty, and an incompatible request is denied before any state mutation. The outcome reports whether it was replayed.
- The canonical promotion plan record is now `molten.world-promotion-plan.v2` and carries the request ref, attempt ref, and generation alongside the operation ref.

## Executed checks

- `cargo test -p molten-core world_promotion`: 11 passed, zero failed. Adds role round-trip, incompatible-request denial, and distinct-operations-with-equal-payloads cases.
- `cargo test -p molten --lib world_promotion`: 11 passed, zero failed. Adds durable role read-back after reopen, replay with the original identity, incompatible-parameter denial after reopen with unchanged head and reservation set, and the legacy record replaying as retained uncertainty.
- `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings` pass.
- `cargo test --workspace --all-targets`: 2,287 passed across 18 suites with zero failures; `cargo test --workspace --doc`: 7 doctests passed.
- Both configured Octet producers reported `clean` with zero findings, warnings and errors: [workspace status](2026-10-21-source/workspace/status.json) and [library status](2026-10-21-source/library/status.json). Artifact import [receipt](2026-10-21-source/workspace/artifact-ledger-receipt.preserves) `blake3:cb989781144f35f1ce92a5d844ea3154bb7652b7d9c4c5f879b716c6b3103b89`; unchanged `strict-ci` gate [receipt](2026-10-21-source/workspace/gate-receipt.preserves) `blake3:42b2880a1a629370e75f163155973ed81ae2b059c1e2e084b3752ee9392721f6`; [remediation plan](2026-10-21-source/workspace/remediation-plan.preserves) `blake3:083c9ac65bf55fd28cac3e3643f82446cccff868508138599ab91bc909de10ad`.
- `nix flake check -L --keep-going --no-update-lock-file` exited 0 with `all checks passed!` for the exported `x86_64-linux` checks. The hermetic nextest check realized 1,810 testcases with zero failures; its [CI receipt](/nix/store/wcrrihv863nmz7s3nf74x2xhrr22qq2d-molten-nextest/ci-test-run-receipt.preserves) records `decision="pass"`, source ref `blake3:3a2c418cafc4b2f0115665c595f6d11897753c0761eef7025526d62859d6d95a`, and raw receipt-file BLAKE3 `eee5604b952bc667ad489268901a53173ab0fe9183de8aeae441b65e121a4b3f`. The raw digest is not the canonical receipt reference. The omitted incompatible systems were `aarch64-darwin`, `aarch64-linux`, and `x86_64-darwin`.

Preserved failures during this work: the first Octet pass reported three `excessive_file_length` and two `function_length` findings, repaired by moving the identity records into `records/identity.rs`, splitting the promotion service into named helpers, and moving the new regressions into a separate test part. A later build failed against a stale `molten-core` artifact in the shared compiler cache; `cargo clean -p molten-core` and a rebuild cleared it, and the full matrix then passed.

## Preserved limits

- Identity equality never proves exactly-once execution of an external effect. A lost response keeps the original identities and the recorded uncertainty; it does not prove the effect happened once.
- The legacy path replays by logical operation identity alone because those records carry no canonical request identity. Their historical detail is unavailable, and the replay reports uncertainty rather than inventing an outcome.
- The pre-change observation is a local store fixture, not a live promotion fault.
- Nothing here changes authority, completion-owner, or head-authority checks; head authority remains owned by the world-head contract.

## Native lifecycle acceptance

- Strict validation passed with 33 active changes, zero issues and zero findings; receipt `fab4ef8a252e0b435c40654928e2bf38c4f801364e2ec099c7b23d9fdc66446d`.
- Proposal, design and tasks gates returned `PASS`: `d44b09d82bd8294adebd4d26e41c5ea81832497150538a9e3d0d7590a4e96495`, `e7c17edf83521b777fbf07158238c76a2e3275ff1a35c109cd8dcc9b761fc1be`, `81511a653b486832af2cd948daab0daa06d3f5b1d29a364000bc2468f3f063de`. The [completeness review](2026-10-21-review-completeness.json) returned `PASS`, receipt `eee4217beb16335bb4dbc94f3c8810051d46da22b2ddf2b1bd9ad226a4fa8ccc`.
- [Sync](2026-10-21-sync-receipt.json) merged the delta spec into `.cairn/specs/world-commit/spec.md` with `blocked=false`, receipt `d7990832233ece61e8de4752a9c4f1f11991f47e69bde35bb7141682682bfe63`.
- [Archive](2026-10-21-archive-receipt.json) reported `blocked=false`, receipt `02613b364861738ee180a03936fc5607ca776f2fd86e4f2daaa6c077d3477b59`; the change now lives at `.cairn/archive/1970-01-01-bind-promotion-logical-identity-across-restarts/`.
- Post-archive strict validation passed with 32 active changes, zero issues and zero findings, receipt `ad3c57e0c2448755af801924b18707da734d1986db138d0bff5b05546994bcc7`.

These structural results do not replace the executed behavior evidence above.
