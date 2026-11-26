## Phase 1: Implementation

- [x] [parallel] Make rotation overlap verification-only in `crates/molten-core/src/fabric_crypto_identity/{rotation,model}.rs`, with tests showing overlap signing is denied and overlap verification still passes. r[molten.crypto_identity.overlap_verification_only]
- [x] [parallel] Validate the deadline domain in `plan_retry` and add tests showing a wall-clock base is denied and a monotonic base is accepted. r[molten.fabric_time.retry_deadline_domain]
- [x] [parallel] Bind `capability_ref` into versioned entropy evidence and reject mismatched references on replay, with positive and negative tests. r[molten.fabric_time.entropy_capability_evidence]
- [x] [parallel] Classify a missing `/proc/self/fd` descriptor bridge as `Unsupported`, with tests for both the missing-bridge and the present-bridge cases. r[molten.testing.descriptor_bridge_unsupported]
- [x] [parallel] Version the snapshot receipt check label to `readable-assertions-subset-admitted`, with tests showing the label matches the evaluated predicate. r[molten.runtime_spine.snapshot_receipt_check_label]
- [x] [parallel] Add the opaque handle reference to the versioned canonical public key status record, with tests showing the field round-trips and carries no key material. r[molten.crypto_identity.status_handle_reference]
- [x] [parallel] Make `CapabilityContext::default()` deny everything, migrate each `Default` caller to explicit grants or `allow_all()`, and add tests showing the default denies and an explicit grant admits. r[molten.runtime_spine.capability_context_deny_by_default]

## Phase 2: Validation

- [x] [serial] Remove the seven discrepancy notes from `docs/technical/` and the README list, then run focused tests, Clippy, Octet, documentation-link checks, and strict Cairn validation. r[molten.crypto_identity.overlap_verification_only] r[molten.crypto_identity.status_handle_reference] r[molten.fabric_time.retry_deadline_domain] r[molten.fabric_time.entropy_capability_evidence] r[molten.testing.descriptor_bridge_unsupported] r[molten.runtime_spine.snapshot_receipt_check_label] r[molten.runtime_spine.capability_context_deny_by_default]

## Verification (2026-11-26, CompanionDocs)

Design notes from implementation:
- `KeyCurrentness::permits_signing` now accepts only `Current`; a new `permits_verification` (Current, Overlap) backs `evaluate_verification`. Overlap signing fails with `HandleNotCurrent(Overlap)`.
- Entropy events use record `fabric-time-entropy-event-v2` and schema `molten.fabric-time.entropy-event.v2` (the entropy port's advertised output schema moves to v2). Every other event kind keeps its earlier label, and `named_event` refuses `EventKind::Entropy`. `replay_fixture_entropy` returns the typed `EntropyReplayError::Entropy(EntropyError::CapabilityMismatch)`. The executable fixture's deterministic entropy scenario publishes its events only after they pass that replay.
- Snapshot receipts use predicate `molten.trellis-runtime.snapshot-authority.v2`. The harness receipt parser accepts v1 and v2. `parse_snapshot_authority_check` reads the old label only in v1 receipts, as an alias.
- Key status with a handle reference uses `fabric-crypto-status-v2` / `molten.crypto-identity.status.v2`. A status without one keeps the v1 bytes. The ledger kind registry maps v2 to `fabric-crypto-status`, so v2 keeps the `audit-receipt` retention class. No other registry keys on the versioned labels. The file adapter builds status from the resolved handle. Adapter `p001`/`p002` were split into `p004` (revoke) and `p005` (revocation records) to keep them under 300 lines.
- `CapabilityContext` `Default` review: there are no direct `default()` callers. The only `derive(Default)` container is harness `SuiteFixtures`. A suite without a capabilities fixture is already rejected by the runner and by report validation (`capabilities_explicit == false`), so the empty default there is the intended deny-by-default and was left unchanged.

Commands (worktree synced to leviathan with `aremote`):
- `cargo check --workspace --all-targets`: pass, no warnings.
- `cargo test -p molten-core --lib fabric_crypto_identity` (14 passed), `fabric_time` (65 passed), `coordination_delivery` (15 passed); `cargo test -p molten-runtime-admission-core` (6 passed).
- `cargo test -p molten --lib` with filters `fabric_crypto_identity` (21 passed, 1 ignored), `fabric_time` (57 passed), `runtime::` (22 passed), `harness` (15 passed), `world_snapshot` (19 passed), `descriptor_bridge` (2), `snapshot_receipt` (3), `vat_snapshot_fixture` (1), `default_runtime_capability` (1), `status_handle`/`status_without_a_handle` (2): all pass. With filter `deterministic`, 56 passed and 1 failed: `inventory_api::tests::hegel_mcp_calls_are_deterministic_and_readonly`, a catalog MCP hegel property that panics inside the hegel runner. This change does not touch that code.
- `cargo clippy -p molten -p molten-core -p molten-runtime-admission-core --all-targets -- -D warnings`: pass.
- `cargo octet check --artifact-dir target/octet`: the first run flagged only the new entropy test module (`underscore_in_module_filename`, `path_segment_repetition`). After renaming it to `src/fabric_time/tests/entropyevidence.rs` and applying the review fixes, a rerun reports zero findings in files this change touched (exit 0).
- Review fixes: `cargo test -p molten --lib fabric_time` 58 passed (adds `fixture_entropy_replays_against_its_admitted_stream`, `mismatched_capability_is_rejected_on_replay_with_the_typed_diagnostic`, `unbound_or_tampered_evidence_is_rejected`); `fabric_crypto_identity` 21 passed (1 ignored); `versioned_crypto_status_keeps_its_kind_and_audit_retention_class` passes; `cargo test -p molten-core --lib fabric_` 230 passed; clippy `-D warnings` clean.
- `python3 scripts/check-doc-links.py`: 320 files, 0 findings.
- `cairn gate proposal|design|tasks repair-companion-doc-source-discrepancies`: valid. Strict `cairn validate` reported one error at the time, in another change's notes (an unknown node-host crate-boundary reference in `fix-authority-bypasses-and-sandbox-isolation/tasks.md`), since fixed on the integration branch. This change adds no errors.
