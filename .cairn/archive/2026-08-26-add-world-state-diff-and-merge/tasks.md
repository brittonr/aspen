## Phase 1: Diff and admission core

- [x] [depends:introduce-world-commit-core] [depends:add-world-branch-head-protocol] Record baseline world-commit, Choregraph history, typed-value, and schema identity checks. r[molten.world_merge.verification]
- [x] [serial] Define root-diff, merge-profile, merge-mode, base/source set, handler profile, conflict, result, bound, and diagnostic DTOs. r[molten.world_merge.diff] r[molten.world_merge.admission]
- [x] [depends:world-merge-dtos] Implement pure root comparison, ancestry-input validation, closed-mode admission, and default denial for runtime-sensitive roots. r[molten.world_merge.diff] r[molten.world_merge.admission]
- [x] [depends:schema-migration-core-publication] Add exact schema and explicit migration-plan admission without executing migrations in the merge core. r[molten.world_merge.admission]

## Phase 2: Merge handlers and publication

- [x] [depends:world-merge-admission] Implement identical-only, ancestor-replacement, and keyed durable-value merge reducers with bounded conflict output. r[molten.world_merge.handlers] r[molten.world_merge.conflicts]
- [x] [depends:world-merge-admission] Add exact immutable application-handler profiles and a pure injected handler boundary. r[molten.world_merge.handlers]
- [x] [parallel] Add canonical Preserves diff, conflict, plan, result, and detached receipt schemas. r[molten.world_merge.diff] r[molten.world_merge.conflicts]
- [x] [depends:world-merge-handlers] Add shell object loading, migration materialization, handler dispatch, output-root persistence, and final merge-commit publication. r[molten.world_merge.result]
- [x] [depends:world-merge-result-publication] Add operator diff, merge-plan, conflict-inspect, and merge-publish commands. r[molten.world_merge.diff] r[molten.world_merge.result]

## Phase 3: Verification and documentation

- [x] [parallel] Add positive equal-root, one-sided replacement, disjoint keyed-value, explicit conflict-resolution, and pure-handler fixtures. r[molten.world_merge.verification]
- [x] [parallel] Add negative missing or ambiguous base, incompatible schema, absent migration, stale handler, unknown mode, task merge, authority merge, effect-attempt merge, opaque snapshot merge, bound exhaustion, handler effect request, conflict, and partial-publication fixtures. r[molten.world_merge.verification]
- [x] [serial] Document supported merge modes, application ownership, schema gates, default-denied roots, and correctness non-claims. r[molten.world_merge.admission] r[molten.world_merge.handlers]
- [x] [depends:world-merge-verification] Run focused tests, Choregraph and schema-core compatibility fixtures, Octet, Clippy with warnings denied, Cairn gates, lifecycle checks, and relevant Nix checks. r[molten.world_merge.verification]
