# Tasks: Promote message passing to the Molten runtime boundary

## Contract and inventory

- [ ] [serial] Inventory independent state owners, production ingress paths, callback groups, message types, effect-plan types, shell scopes, adapter scopes, composition roots, and live runtime-handle providers. r[molten.message_boundary.contract] r[molten.message_boundary.static_admission]
- [ ] [serial] Define the typed Nickel message-boundary profile, protocol-selected message fields, finite bounds, adapter scopes, scheduler obligations, evidence roles, and non-claims. r[molten.message_boundary.contract] r[molten.message_boundary.claim_boundary]
- [ ] [parallel] Add positive complete profile fixtures and negative missing-owner, missing-bound, unknown-field, floating-ref, handle-provider, and claim-promotion fixtures. r[molten.message_boundary.contract] r[molten.message_boundary.verification]

## Core message and transition path

- [ ] [serial] Add canonical owned boundary-message types and BLAKE3 identities without transport or vendor handles. r[molten.message_boundary.contract]
- [ ] [serial] Route initialize, start, request, message, stream, timer, health, checkpoint, recovery, drain, shutdown, and effect-completion inputs through canonical callback envelopes. r[molten.message_boundary.callback_envelope]
- [ ] [serial] Make each declared state-owner transition consume explicit state and an inbound message and return explicit state, events, outbound messages, decisions, or effect plans. r[molten.message_boundary.transition_shape]
- [ ] [serial] Keep logical session and stream identifiers and finite phases as handle-free core values. r[molten.message_boundary.connection_events]

## Shells, adapters, and scheduling

- [ ] [serial] Confine Iroh, socket, channel, client, executor, task, and borrowed-buffer values to declared shell and adapter scopes. r[molten.message_boundary.handle_containment]
- [ ] [parallel] Convert live connection open, close, reset, retry, overload, cancellation, and uncertain delivery observations into canonical messages or adapter diagnostics. r[molten.message_boundary.connection_events]
- [ ] [serial] Route message delivery, timers, storage completions, process lifecycle, fault activation, authority changes, resource outcomes, and other modeled choices through the bounded deterministic scheduler. r[molten.message_boundary.scheduler_closure]
- [ ] [serial] Prohibit deterministic adapters from invoking state-owner transitions outside scheduler-visible delivery. r[molten.message_boundary.scheduler_closure]
- [ ] [parallel] Run shared live and deterministic adapter conformance over canonical application traces and explicit declared differences. r[molten.message_boundary.transport_parity] r[molten.message_boundary.same_core]

## Octet admission and roadmap compatibility

- [ ] [depends:molten.message_boundary.static_admission] [serial] Pin the published Octet message-boundary revision through Nix-generated lock updates and select strict architecture admission. r[molten.message_boundary.static_admission]
- [ ] [serial] Declare Molten state owners, core scopes, message types, transition paths, effect plans, adapter scopes, handle providers, and composition roots in the Octet architecture policy. r[molten.message_boundary.static_admission]
- [ ] [parallel] Add compiler-backed positive and negative Molten fixtures for direct, aliased, nested, associated, async, callback, borrowed-buffer, channel, shared-state, and vendor-handle paths. r[molten.message_boundary.static_admission] r[molten.message_boundary.verification]
- [ ] [serial] Audit active Cairns for connection wake, stream, retry, session, shared-state, and adapter behavior, then add narrow compatibility updates where required. r[molten.message_boundary.roadmap_compatibility]

## Evidence and closeout

- [ ] [parallel] Add deterministic replay, scheduler closure, first-divergence, same-core identity, multiprocess live, and differential evidence with separate roles. r[molten.message_boundary.evidence] r[molten.message_boundary.claim_boundary]
- [ ] [parallel] Add negative fixtures for handle escape, shared-state bypass, borrowed-buffer escape, callback bypass, hidden retry, scheduler bypass, same-core drift, semantic drift, stale generation, overload, cancellation, and uncertain delivery. r[molten.message_boundary.verification]
- [ ] [serial] Run focused tests before and after core changes, then formatting, strict Clippy, strict Octet, Cairn validation and gates, Tracey coverage, and the smallest relevant Nix checks. r[molten.message_boundary.verification]
- [ ] [serial] Preserve static, simulation, live, host-chaos, VM, hardware, correctness, delivery, availability, security, and release-readiness non-claims before sync or archive. r[molten.message_boundary.claim_boundary]
