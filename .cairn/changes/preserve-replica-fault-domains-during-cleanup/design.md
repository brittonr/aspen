# F05 design

## Boundary and source

Source revision: `fa1ced3e808861d8ce59f02a6fd6b13b655f5147`.

`crates/molten-core/src/content_replication/action.rs::plan_cleanup` uses reverse peer-ID order and takes the excess count. It does not evaluate residual domains. `src/content_replication/service/execution.rs::execute_cleanup` separately authorizes retention cleanup before the content port call.

The accepted content-replication requirements already separate placement policy from retention and deletion authority. This change closes the planner gap without moving policy into adapters.

## Proposed decision

The pure core maintains a proposed residual replica set. It evaluates each candidate against that set before adding a typed cleanup action. Both count thresholds and minimum fault domains remain satisfied after every selected removal. Deterministic ordering remains a tie-breaker, not a policy override.

An unsafe candidate is skipped. Another eligible candidate can satisfy cleanup demand. If none exists, the planner retains excess replicas and reports a bounded unresolved cleanup cause. It never invents clearance or removes pins.

The shell executes only admitted actions. It retains generation, authority, clearance, and pin validation before cleanup. Adapter rejection preserves content and retention state and produces no successful deletion observation. No new dependency or internal-function port is necessary.

## Compatibility and receipts

Prefer existing action and policy schemas. Corrected plans can have different action identities and canonical plan references because the selected target changes. Parent review must settle replay treatment for old unsafe plans: historical evidence remains readable, but cannot authorize fresh cleanup without current admission.

A planning receipt describes candidate selection only. A deletion observation requires successful shell execution and separate authority. No retrospective claim can promote the F05 counterexample into executed deletion.

## Coordination and order

F04 repairs current availability accounting. F07 preserves unresolved demand in status. F05 uses current verified replicas and preserves domains in the proposed remainder. Keep each package independent and separately testable.

Recommended integration order is F04, F07, then F05. This order reduces shared-file conflicts, but creates no blocking dependency. Combined tests must reject cleanup based on historical-only replicas and preserve partial status after adapter denial.

## Validation and ownership

The content-replication maintainers own planner, shell, adapters, and docs. Run focused existing cleanup tests before core edits. Move `audit_cleanup_preserves_required_fault_domains` into normal tests with named fixture values.

Cover the three-peer trigger, multiple removals, reordered peers, unique domains, pins, missing authority, stale clearance, and no safe candidate. Controlled adapters must record authorization before cleanup and no deletion after rejection. Test both accepted and rejected serialization and port results.

Run focused Octet and Clippy, workspace tests, relevant Nix conformance checks, and all required Cairn gates without weakening checks. No gate result is asserted by this design.
