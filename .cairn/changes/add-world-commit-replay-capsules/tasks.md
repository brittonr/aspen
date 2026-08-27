## Phase 1: Trace and capsule cores

- [ ] [depends:introduce-world-commit-core] Record baseline replay-summary, sealed-bundle, content-manifest, restore-profile, and import/export tests. r[molten.world_replay.verification]
- [ ] [serial] Define bounded transition-trace, transition-step, expected-successor, capsule-manifest, member-role, protection-profile, replay-plan, divergence, and receipt DTOs. r[molten.world_replay.transition_chain] r[molten.world_replay.capsule]
- [ ] [depends:world-replay-dtos] Implement canonical Preserves codecs and domain-separated BLAKE3 identities for traces, capsules, divergence records, and receipts. r[molten.world_replay.transition_chain] r[molten.world_replay.capsule]
- [ ] [depends:world-replay-dtos] Implement pure trace validation, ancestry checks, bound checks, complete typed-closure planning, and deterministic operation ordering. r[molten.world_replay.transition_chain] r[molten.world_replay.capsule]
- [ ] [parallel] Implement pure expected-versus-actual commit comparison with earliest step, typed-root, and bounded field-path divergence. r[molten.world_replay.divergence]

## Phase 2: Shell and adapters

- [ ] [depends:world-replay-plan] Add narrow content materialization, profile restore, bounded transition execution, successor capture, import publication, and receipt ports. r[molten.world_replay.execution_boundary] r[molten.world_replay.import]
- [ ] [depends:world-replay-ports] Implement logical-profile replay through existing deterministic Molten runtime adapters. r[molten.world_replay.transition_chain]
- [ ] [depends:world-replay-ports] Implement capsule export and import with existing content manifests, sealed reproduction bundles, and content-exchange adapters. r[molten.world_replay.capsule] r[molten.world_replay.import]
- [ ] [depends:portable-chaoscontrol-snapshot-descriptor] Add the exact opaque-profile adapter without claiming logical equivalence or cross-cohort portability. r[molten.world_replay.execution_boundary]
- [ ] [parallel] Add detached Valence-friendly replay and import receipts that preserve profile, horizon, closure, redaction, and authority non-claims. r[molten.world_replay.receipts]

## Phase 3: Verification and documentation

- [ ] [parallel] Add positive multi-step logical replay, stable repeated replay, complete capsule export/import, deduplicated member, and exact opaque-profile fixtures. r[molten.world_replay.verification]
- [ ] [parallel] Add negative wrong parent, reordered step, wrong successor, earliest divergence, missing member, extra undeclared member, tampered bytes, noncanonical codec, unsupported profile, stale schema, plaintext secret, bearer capability, unavailable key, and import-as-authority fixtures. r[molten.world_replay.verification]
- [ ] [serial] Document transition semantics, capsule closure, profile limits, protection profiles, detached authority, and the absence of universal replay claims. r[molten.world_replay.receipts]
- [ ] [depends:world-replay-verification] Run focused tests, Octet, Clippy with warnings denied, Cairn validation and gates, lifecycle checks, and relevant Nix checks. r[molten.world_replay.verification]
