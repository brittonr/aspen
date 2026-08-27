## Phase 1: World closure and retention core

- [ ] [depends:introduce-world-commit-core] [depends:add-world-branch-head-protocol] Record baseline content, DAG, Iroh, root-inventory, retention, remote-clearance, and GC checks. r[molten.world_distribution.verification]
- [ ] [depends:dag-sync-system-extension] Implement a pure world-commit DAG projection with typed edges, deterministic missing closure, cycle handling, and exact bounds. r[molten.world_distribution.closure]
- [ ] [depends:content-replication-system-extension] Add immutable world-object transfer plans that preserve generic replication ownership and reject unsolicited closure expansion. r[molten.world_distribution.closure] r[molten.world_distribution.partial]
- [ ] [serial] Define detached head-claim exchange, peer observation, conflict-set, retention-root, lease, reachability, and diagnostic DTOs. r[molten.world_distribution.head_claims] r[molten.world_distribution.retention_roots]
- [ ] [depends:world-distribution-dtos] Implement pure local claim-set admission and complete retention-root projection. r[molten.world_distribution.head_claims] r[molten.world_distribution.retention_roots]
- [ ] [depends:artifact-binding-semantic-effects] Map complete world roots into Artifact Binding Core reachability without transferring deletion authority. r[molten.world_distribution.gc_boundary]

## Phase 2: Transport and retention shell

- [ ] [depends:world-dag-projection] Add DAG-sync and content-replication adapters for immutable commits and typed roots. r[molten.world_distribution.closure] r[molten.world_distribution.partial]
- [ ] [depends:world-head-claim-exchange] Add a bounded detached claim exchange that authenticates statements and applies current local policy before selection. r[molten.world_distribution.head_claims]
- [ ] [depends:world-distribution-adapters] Add resumable operation state, partial closure inspection, corruption rejection, and activation blocking. r[molten.world_distribution.partial]
- [ ] [depends:world-retention-projection] Add observation-only retention and pin-path reports for heads, executions, replay, merges, promotions, reconciliation, holds, leases, and incomplete transfers. r[molten.world_distribution.retention_roots]
- [ ] [depends:world-retention-reports] Integrate reports into existing retention planning while preserving policy, clearance, apply, execute, audit, and deletion gates. r[molten.world_distribution.gc_boundary]
- [ ] [depends:world-distribution-adapters] Add operator sync-plan, sync, resume, closure-inspect, claims-inspect, pins-inspect, and retention-explain commands. r[molten.world_distribution.partial] r[molten.world_distribution.gc_boundary]

## Phase 3: Verification and documentation

- [ ] [parallel] Add positive complete closure, deduplicated transfer, interrupted resume, authenticated claim exchange, conflict preservation, and explainable retention fixtures. r[molten.world_distribution.verification]
- [ ] [parallel] Add negative partial closure, wrong-domain object, corrupt bytes, cycle, bound exhaustion, unsolicited object, stale claim, unauthorized claim, competing heads, stale lease, missing execution root, outbox pin omission, uncertain remote state, reachability-as-deletion, and convergence-overclaim fixtures. r[molten.world_distribution.verification]
- [ ] [serial] Document immutable and mutable protocols, activation gates, retention roots, remote uncertainty, and availability non-claims. r[molten.world_distribution.gc_boundary]
- [ ] [depends:world-distribution-verification] Run focused tests, DAG-sync, replication, Artifact Binding, and retention compatibility fixtures, Octet, Clippy with warnings denied, Cairn gates, lifecycle checks, and relevant Nix checks. r[molten.world_distribution.verification]
