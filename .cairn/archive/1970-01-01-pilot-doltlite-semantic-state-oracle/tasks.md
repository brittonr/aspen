## Phase 1: Source and contract foundation

- [x] [depends:introduce-world-commit-core] Record baseline semantic-state, branch, merge, concurrency, retention, and benchmark outcomes before adding the oracle. r[molten.world_state_oracle.verification]
- [x] [serial] Pin DoltLite commit `10170ed82c1b12414db8d1b29d2fe9ea2a72fd88`, preserve applicable notices, record imported scope, and disable remotes. r[molten.world_state_oracle.source]
- [x] [serial] Define the typed Nickel compatibility ledger, closed statuses, required evidence, issue fields, and non-increasing exception policy. r[molten.world_state_oracle.compatibility]
- [x] [parallel] Define canonical semantic-state test rows and normalized BLAKE3 observation records without backend-global identity claims. r[molten.world_state_oracle.observations]

## Phase 2: Oracle adapter and cases

- [x] [depends:doltlite-oracle-contracts] Add a narrow test-owned `SemanticStateOracle` port and keep SQLite and DoltLite types outside Molten cores. r[molten.world_state_oracle.boundary]
- [x] [depends:doltlite-source-pin] Implement the disposable capability-rooted adapter with explicit primary keys, deterministic ordering, bounds, and cleanup. r[molten.world_state_oracle.boundary]
- [x] [parallel] Add history-independent state, detached read, branch isolation, successful compare-and-advance, reader-safe GC, exact reopen, and serialization cases. r[molten.world_state_oracle.behavior]
- [x] [parallel] Add rowid, custom-collation, stale writer, competing writer, missing pin, tamper, wrong format, malformed serialization, remote, and unsupported-operation cases. r[molten.world_state_oracle.verification]
- [x] [serial] Record complete-world atomicity, durable conflicts, typed merge, authority, effects, and stack-global identity as intentional Molten-owned differences. r[molten.world_state_oracle.compatibility]

## Phase 3: Differential evidence and closeout

- [x] [depends:doltlite-oracle-cases] Publish normalized observations to the Prolly and benchmark rails without treating agreement as correctness proof. r[molten.world_state_oracle.observations]
- [x] [parallel] Add positive and negative fixture mutation tests for missing evidence, status drift, exception growth, and overclaiming. r[molten.world_state_oracle.compatibility] r[molten.world_state_oracle.verification]
- [x] [serial] Document source, licenses, supported cases, intentional differences, resource bounds, and production non-goals. r[molten.world_state_oracle.source]
- [x] [depends:doltlite-oracle-verification] Run focused tests, Octet, Clippy with warnings denied, Cairn validation and gates, and relevant Nix checks. r[molten.world_state_oracle.verification]
