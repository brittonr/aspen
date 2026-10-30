## Phase 1: Federation extension and signed models

- [x] [serial] Define the federated-pull system-extension manifest, peer configuration, protocol/session, announcement, inventory, delegate, request/response, conflict, status, and evidence records. r[molten.federated_pull_runtime.manifest]
      `src/federation/admission/mod.rs` (`admit_manifest`, `validate_bindings`) and `crates/molten-core/src/federation/{model,session,planner}.rs`; records in `src/federation/envelope/records.rs`.
- [x] [serial] Replace production federation fixture signatures with purpose-separated cryptographic adapter operations over canonical payload refs while retaining fixture profiles for simulation only. r[molten.federated_pull_runtime.crypto]
      `src/federation/envelope/signatures.rs` signs/verifies through `fabric_crypto_identity::{sign_federation_payload, admit_federation_verification}`; `production_signing_rejects_fixture_profile_even_with_a_real_key_handle` passes.
- [x] [parallel] Add positive and negative signature tests for current key, wrong key, wrong purpose/domain, stale generation, revocation, malformed records, and redaction. r[molten.federated_pull_runtime.crypto]
      `signed_records_deny_wrong_key_purpose_context_generation_revocation_and_corruption`, `signed_announcement_binds_resource_and_rejects_wrong_key`, `tampered_inventory_signature_rejects_before_import` and `fabric_crypto_identity::tests::redaction_keeps_public_status_and_denies_private_material` pass in the full suite.

## Phase 2: Discovery and receiver-owned pull

- [x] [serial] Implement static configured peers as the initial discovery profile and represent optional endpoint, gossip, tracker, DHT/pkarr-style, probe, and catalog inputs as hint-only candidate locators. r[molten.federated_pull_runtime.discovery_hints]
      `observed_locator_cannot_substitute_for_static_peer_admission`, `iroh_locator_records_are_hint_only_and_admission_requires_verification`, `pkarr_locator_pointer_is_optional_and_stale_denies` pass.
- [x] [serial] Implement pure peer selection, inventory diff, missing-set, DAG/content strategy, freshness, rate, resource, conflict, and local-admission prerequisite planning. r[molten.federated_pull_runtime.receiver_pull] r[molten.federated_pull_runtime.bounds_freshness] r[molten.federated_pull_runtime.conflict_boundary]
      `crates/molten-core/src/federation/planner.rs` (missing set, canonical `Conflict` evidence); `divergence_is_reported_without_fetching_or_overwriting`, `rate_limit_denies_inventory_before_fetch` pass.
- [x] [parallel] Reject push import, unsolicited content, oversized inventory, stale signatures, unavailable peers, and unadmitted domain merges before trusted local mutation. r[molten.federated_pull_runtime.discovery_hints] r[molten.federated_pull_runtime.receiver_pull] r[molten.federated_pull_runtime.conflict_boundary]
      `hegel_receiver_driven_sync_no_push_and_verify_before_import`, `receiver_policy_denial_does_not_import_remote_resource`, `distributed_federation_malicious_signer_stale_inventory_and_resource_exhaustion` pass; live `unsolicited-push` rails deny (evidence/2026-10-30-live-rails.md).

## Phase 3: Executable anti-entropy service

- [x] [serial] Host bounded anti-entropy sessions as a supervised system extension using crypto, transport, DAG, content, time, resource, and observability bindings. r[molten.federated_pull_runtime.manifest] r[molten.federated_pull_runtime.bounds_freshness]
      `src/federation/lifecycle/{mod,extension,scheduling,recovery}.rs` host the supervised extension; DAG effects route through `src/federation/transfer/execution.rs`; resume/partition/restart fences pass in `distributed_federation_*` tests.
- [x] [serial] Fetch through DAG/content extensions, verify canonical identity, and invoke local schema, policy, capability, provenance, resource, retention, and import admission before registry mutation. r[molten.federated_pull_runtime.receiver_pull]
      `src/federation/transfer/{dagnetwork,import,quarantine}.rs` and `admission/rules.rs`; `distributed_federation_preflights_all_import_evidence_before_fetch` passes and the live rails commit only after verification.
- [x] [parallel] Add local status assertions and bounded operator readback for peers, freshness, sessions, inventories, missing sets, fetches, verification, admission, denial, conflicts, resources, and evidence. r[molten.federated_pull_runtime.status_evidence]
      `src/federation/lifecycle/status.rs`; `sync_status_assertions_capture_imports_and_denials` and `distributed_federation_denial_survives_full_evidence_budget` pass.

## Phase 4: Simulation, live tests, and validation

- [x] [parallel] Run the same federation core under deterministic simulation with malicious-peer, stale inventory, partition, restart, resource exhaustion, signature, corruption, conflict, and unsolicited-push faults. r[molten.federated_pull_runtime.final_validation]
      `src/federation/simulation.rs`, `simulation/adversarial.rs`, `simulationworld.rs`; the live receiver executes the installed simulation campaign and plan parity on every live run (evidence/2026-10-30-live-rails.md).
- [x] [parallel] Add live-loopback and local multiprocess signed pull/import fixtures with offline-verifiable receipts. r[molten.federated_pull_runtime.final_validation]
      `molten federation live-loopback|distinct-process|verify-detached`; positive, five-fault and four explicit ignored live tests executed (evidence/2026-10-30-live-rails.md) under an explicitly experimental DAG conformance selection.
- [x] [serial] Run focused federation, crypto, transport, DAG/content, admission, rate/resource, simulation/live parity, no-push-import, and conflict-boundary tests. r[molten.federated_pull_runtime.final_validation]
      Full workspace suite 2,904 passed / 0 failed / 28 ignored, including all federation, `mesh::`, `dag_sync::` and crypto tests; see evidence/2026-10-30-verification.md.
- [x] [serial] Run formatting, Clippy, Cairn validation, proposal/design/tasks gates, and the smallest relevant Nix checks before sync and archive. r[molten.federated_pull_runtime.final_validation]
      See `evidence/2026-10-30-verification.md`, `evidence/2026-10-30-live-rails.md` and `review.md`.

## Prerequisite status

The former transitive consistency blocker is stale. Authoritative packages are
archived under `.cairn/archive/`: DAG sync on 2026-08-26, whole-system simulation
on 2026-08-01, consistency and cross-process transport on 2026-07-24,
cryptographic identity, content store, observability and transport sessions on
2026-07-12, and time scheduling and the extension runtime on 2026-07-11.

Archive status is not production admission. Live federation still requires its
own reviewed manifest, current authority and resource reservations, actual
remote DAG/content delivery, local import gates, and recorded distinct-process
evidence. `DagFabricTransportAdapter` is a conformance fixture with fixed
admission refs and request-metadata loopback; it is not a production content
fetch adapter. Filesystem loopback is not live networking. No task above is
complete merely because a prerequisite package was archived.
