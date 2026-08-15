## Phase 1: Schemas and reference boundary

- [x] [serial] r[molten.effects.reference_boundary] Document Bluefin as non-normative prior art and avoid Haskell/Bluefin API, package, effect-row, or implementation compatibility claims.
- [x] [serial] r[molten.effects.handler_binding_schema] Define canonical acyclic `handler-binding-v1` evidence with profile, scope, implementation, policy/capability/authority refs, resource refs, operation set, evidence refs, and checks; handles point at binding refs rather than the binding listing child handle refs.
- [x] [serial] r[molten.effects.handle_schema] Define canonical `effect-handle-v1` evidence with kind, scope, handler-binding ref, operation set, authority/resource refs, validity, transfer policy, parent handle, evidence refs, and checks.
- [x] [parallel] r[molten.effects.handle_not_authority] Specify and test that handle refs identify effect surfaces but never grant authority without capability/authority/policy/resource evidence.

## Phase 2: Request binding and validation

- [x] [serial] r[molten.effects.request_handle_ref] Add `handle_ref` binding to evidence-bearing executor hostcall request envelopes while preserving canonical replay identity; generic runtime effect-request handles remain for adapter/effect integration.
- [x] [serial] r[molten.effects.introduction_before_use] Validate handle introduction-before-use, handler-binding availability, deterministic handle identity, and canonical artifact refs.
- [x] [serial] r[molten.effects.scope_lifetime_validation] Validate run/session/actor/turn scope, not-before/expiry, revocation refs, and stale-handle denial before side effects.
- [x] [parallel] r[molten.effects.operation_authorization_binding] Validate that requested operation, capability context, authority context, policy refs, and resource grant/consumption refs match the handle and handler binding.

## Phase 3: Same-kind effects and compound handlers

- [x] [serial] r[molten.effects.same_kind_disambiguation] Add tests and fixtures with multiple same-kind handles in one actor/session/turn, requiring handle refs rather than effect-kind inference.
- [x] [serial] r[molten.effects.compound_handler_profiles] Model compound handler profiles that expose multiple child handles with shared policy/capability/resource evidence and per-handle operation sets.
- [x] [parallel] r[molten.effects.dynamic_operation_records] Represent dynamic handler operations as reviewed adapter/callable refs with canonical request/response evidence.
- [x] [parallel] r[molten.effects.handle_replay_identity] Prove replay preserves handle identity and rejects tampered, substituted, or missing handle refs.

## Phase 4: Remote, transfer, and lifecycle

- [x] [serial] r[molten.effects.local_only_default] Deny transfer or remote use of local-only handles unless explicit attenuation or remote-proxy evidence is present.
- [x] [serial] r[molten.effects.remote_proxy_handles] Bind remote-proxy handles to peer bootstrap agreement, node identity, authority context, feature negotiation, resource limits, and revocation policy.
- [x] [parallel] r[molten.effects.handle_attenuation] Support parent/child handle attenuation with narrower operation sets, narrower scope, shorter expiry, and explicit evidence refs.
- [x] [parallel] r[molten.effects.handle_cleanup_gc] Define cleanup and GC rules that remove live usability while preserving historical artifacts needed for replay.

## Phase 5: Integration and tests

- [x] [serial] r[molten.effects.hostcall_boundary_integration] Integrate handle validation with executor hostcall boundary evidence and pass-evidence gate receipts.
- [x] [serial] r[molten.effects.adapter_integration] Integrate handles with storage/blob/network/remote-sync/replay-record adapter profiles.
- [x] [serial] r[molten.effects.negative_security_tests] Add denial tests for missing handles, stale handles, escaped handles, revoked handles, wrong operation, wrong scope, wrong authority/resource refs, and handle-only authority attempts.
- [x] [parallel] r[molten.effects.property_tests] Add Hegel properties for same-kind disambiguation, introduction-before-use, monotonic attenuation, replay stability, and no-side-effect-before-handle-denial.
