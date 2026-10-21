# F05 executed verification — 2026-10-20

## Outcome, capability, and owner

The content-replication maintainers own this change. The immediate correction retains the only replica in a required fault domain. The durable capability is deterministic cleanup selection over the cumulative remainder, independent of deletion authority.

The pre-edit public planner counterexample selected `peer-c`, the unique second-domain replica in the A/A/B three-peer fixture. The corrected public smoke selected only `peer-b` and printed `residual_domains_preserved=true`. This demonstrates corrected plan selection, not production deletion. The [shared executed record](../../1970-01-01-repair-lost-replicas-after-prior-success/evidence/2026-10-20-verification.md) retains baseline counts, exact commands, public shell observations, common gate results, and failed-attempt evidence.

## Consumer-visible controls

`crates/molten-core/src/content_replication/tests/cleanup.rs` retains the audit case as `audit_preserves_required_fault_domains`; its module-qualified name avoids the required Octet repeated-path-name error. The same module covers all six three-peer input permutations, cumulative multiple removals, pinned/uncleared safe alternatives, no-safe-candidate deferral, unsatisfied domain minima, missing/disabled cleanup authority, stale replicas, and queue limits.

`src/content_replication/tests/cleanup/mod.rs` uses controlled ports that actually remove the selected inventory member after retention admission. It observes A/C remaining, a verified durable operation targeting B, no transfer call, and admission → deletion → durable storage → operation publication → final receipt ordering. `cleanup/rejections.rs` retains content, retention facts, and durable history for active pins, stale clearances, denied/stale/malformed/error admissions, authority/epoch rejection, and content-adapter denial.

A malformed result returned after a successful adapter effect is deliberately different: the member is already removed, but no successful durable operation or observation is published. That test makes no rollback claim. `src/content_replication/tests/replay.rs` also proves that recorded cleanup success does not bypass current retention admission.

F04 current-inventory and F07 partial-receipt controls run beside these tests. Deferred cleanup remains partial even when desired replica counts are already met; historical transfer results do not substitute for current replicas in cleanup selection. No cyclic package dependency was added.

## Observed verification and compatibility

- Initial focused runs: 33 core tests and 28 shell/conformance-filtered tests passed, with zero failures or ignored tests. Later policy-only layout/name/import corrections changed no test bodies.
- The hermetic `world-distribution-octet-deny-all` check passed with zero findings, warnings, or errors after those corrections. No lint allowance was added.
- Final-tree `cargo fmt --check`, the unchanged inherited-debt guard, all nine separate F04/F05/F07 planning gates, and strict owner-policy Cairn validation passed as recorded in the shared record.
- Final configured workspace/library Octet producers were clean, the canonical `strict-ci` source gate passed, default workspace tests passed 2,219 cases, and the compatible-feature all-targets run passed 2,231 cases with only the external DoltLite oracle ignored. Strict workspace/all-targets Clippy also passed for that feature set. Exact commands and canonical receipts are retained in the shared record; all implementation task boxes now reflect executed work without granting sync/archive authority.
- The direct focused Octet wrapper's existing escaping diagnostic-path failure and sibling Cairn Nix path-ingestion failure are retained in that record, not recast as passing checks.

The [compatibility decision](../design.md#implementation-compatibility-decision-2026-10-20) keeps cleanup action/policy fields and operation/status/receipt v1 shapes. New canonical plans are explicitly v2 because F07 adds missing-count bindings. Old unsafe plans remain historical evidence: the public shell reobserves facts, replans, and separately checks current cleanup admission.

These results do not establish production data loss, permanent durability, deletion authority, lifecycle completion, publication, or release readiness.
