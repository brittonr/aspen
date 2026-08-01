# Tasks

## 1. Baseline and dependency admission

- [ ] [serial] Record current fabric-membership, authority, resource, consistency, durable-state, logical-time, system-extension, and simulation boundaries before implementation. r[molten.collaboration_scope.versioned_extension]
- [ ] [serial] Admit the nominal-reference and simulation dependencies, plus exact external Basalt and UCAN consumer identities or bounded blockers. r[molten.collaboration_scope.nominal_model] r[molten.collaboration_scope.authority_snapshot]
- [ ] [parallel] Record the QM source revision, reviewed mechanisms, rejected mechanisms, license facts, and non-transfer claim boundary. r[molten.collaboration_scope.validation]

## 2. Build the functional core

- [ ] [serial] Define nominal scope, subject, membership, resource-binding, audience, request, operation, policy, decision, snapshot, and receipt references with bounded validation. r[molten.collaboration_scope.nominal_model]
- [ ] [depends:nominal-model] Implement pure scope creation and membership add, remove, expiry, and duplicate-operation transitions with monotonic membership epochs. r[molten.collaboration_scope.membership_currentness]
- [ ] [depends:nominal-model] Implement one-owner resource binding and explicit consistency-serialized scope-move transitions. r[molten.collaboration_scope.resource_binding]
- [ ] [depends:membership-currentness,resource-binding] Implement effective-audience intersection over current membership, scope policy, resource policy, and organization-floor facts. r[molten.collaboration_scope.effective_audience]
- [ ] [depends:resource-binding] Implement separate `resource/share` admission and deny inference from read, use, ancestry, or prior delivery. r[molten.collaboration_scope.nontransitive_sharing]
- [ ] [depends:effective-audience] Implement exact snapshot binding, currentness validation, safe diagnostics, and deterministic BLAKE3 receipt payloads. r[molten.collaboration_scope.authority_snapshot] r[molten.collaboration_scope.safe_projection]

## 3. Host the system extension

- [ ] [serial] Add the versioned Preserves manifest and records without changing fabric-membership schemas or treating ordinary actor traffic as collaboration traffic. r[molten.collaboration_scope.versioned_extension]
- [ ] [depends:functional-core] Bind scope mutations to admitted consistency, durable-state, logical-time, resource, authority, and observability ports. r[molten.collaboration_scope.membership_currentness] r[molten.collaboration_scope.authority_snapshot]
- [ ] [depends:system-extension-shell] Supply current Basalt and UCAN decision facts through the shell without storing raw tokens, keys, identity documents, or policy bodies. r[molten.collaboration_scope.authority_snapshot]
- [ ] [depends:system-extension-shell] Add bounded operator status for scope refs, epochs, counts, lifecycle, currentness, and evidence refs without member labels or resource content. r[molten.collaboration_scope.safe_projection]

## 4. Add consumer-safe projections

- [ ] [parallel] Add metadata-only snapshot fixtures for Lattice effect binding, Animus context references, and Tile read-only status. r[molten.collaboration_scope.safe_projection]
- [ ] [parallel] Deny snapshots that contain raw tokens, identity documents, policy bodies, messages, memories, prompts, files, credentials, resource bytes, or unrestricted diagnostics. r[molten.collaboration_scope.safe_projection]
- [ ] [parallel] Document that downstream consumers must revalidate snapshot currentness and cannot treat snapshots as bearer capabilities. r[molten.collaboration_scope.authority_snapshot]

## 5. Verify positive and negative behavior

- [ ] [serial] Add positive tests for each scope kind, member changes, exact resource binding, authorized move, organization-floor narrowing, audience admission, and current snapshot consumption. r[molten.collaboration_scope.validation]
- [ ] [serial] Add negative tests for stale epochs, removed or expired members, one denied audience subject, wrong scope, wrong resource, wrong ability, missing policy, and weakened organization floors. r[molten.collaboration_scope.validation]
- [ ] [serial] Add negative tests for transitive share, implicit copy or move, duplicate conflict, replay, revocation, partition, restart, uncertain durability, stale consumer use, and payload leakage. r[molten.collaboration_scope.validation]
- [ ] [parallel] Add properties for epoch monotonicity, one-owner binding, denial state preservation, deterministic replay, and bounded collection growth. r[molten.collaboration_scope.validation]
- [ ] [parallel] Add whole-system simulation and multiprocess fixtures for concurrent membership mutation, stale admission, restart, partition, and currentness recovery. r[molten.collaboration_scope.validation]

## 6. Validate and close out

- [ ] [serial] Run formatting, focused tests, Clippy, system-extension fixtures, Cairn validation and gates, traceability, and the smallest relevant Nix checks. r[molten.collaboration_scope.validation]
- [ ] [serial] Record immutable schema, policy, Basalt, UCAN, simulation, and consumer evidence identities plus exact skipped-rail blockers. r[molten.collaboration_scope.validation]
- [ ] [serial] Verify documentation preserves identity, authority, revocation, disclosure, transport, confidentiality, and release non-claims before sync and archive. r[molten.collaboration_scope.safe_projection] r[molten.collaboration_scope.validation]
