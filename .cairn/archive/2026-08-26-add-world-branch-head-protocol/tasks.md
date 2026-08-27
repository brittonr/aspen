## Phase 1: Pure branch and claim contracts

- [x] [depends:introduce-world-commit-core] Pin and map the reviewed Choregraph branch-history and Artifact Auth cohorts with exact Cargo and Nix source identities. r[molten.world_heads.claim] r[molten.world_heads.authentication]
- [x] [serial] Define branch identity, branch class, head claim, generation, ancestry, signer observation, conflict set, decision, and diagnostic DTOs. r[molten.world_heads.claim]
- [x] [depends:world-head-dtos] Implement pure claim validation, ancestry checks, Choregraph compare-and-swap mapping, stale-generation rejection, currentness classification, and bounded conflict construction. r[molten.world_heads.cas] r[molten.world_heads.rollback] r[molten.world_heads.conflicts]
- [x] [parallel] Add canonical Preserves statement, conflict-report, and transition-receipt schemas with exact artifact-auth byte compatibility. r[molten.world_heads.authentication]

## Phase 2: Durable shell

- [x] [depends:world-head-core] Add narrow current-head, authenticated-statement, authority-observation, signing, transaction, and reconciliation ports. r[molten.world_heads.cas] r[molten.world_heads.authentication]
- [x] [depends:world-head-ports] Implement local head creation and advance with in-transaction current-head, generation, policy, and authority rechecks. r[molten.world_heads.cas] r[molten.world_heads.rollback]
- [x] [depends:world-head-local-store] Add explicit competing-claim storage and inspection without automatic last-writer selection. r[molten.world_heads.conflicts]
- [x] [depends:world-head-local-store] Add operator plan, sign, inspect, advance, conflict, and reconcile commands with bounded detached receipts. r[molten.world_heads.claim] r[molten.world_heads.authentication]

## Phase 3: Verification and documentation

- [x] [parallel] Add positive root creation, linear advance, authorized merge advance, threshold authentication, and stable repeated-plan fixtures. r[molten.world_heads.verification]
- [x] [parallel] Add negative stale expected head, old or skipped generation, replayed claim, whole-store rollback without an independent witness, unrelated successor, duplicate parent, wrong purpose, unknown signer, threshold miss, revoked signer, stale authority, concurrent claim, uncertain storage, authentication-as-authorization, and anti-rollback-overclaim fixtures. r[molten.world_heads.verification]
- [x] [serial] Document branch ownership, rollback limits, conflict behavior, signer boundaries, and remote-convergence non-claims. r[molten.world_heads.authentication] r[molten.world_heads.conflicts]
- [x] [depends:world-head-verification] Run focused tests, Artifact Auth and Choregraph compatibility fixtures, Octet, Clippy with warnings denied, Cairn gates, lifecycle checks, and relevant Nix checks. r[molten.world_heads.verification]
