## Why

Node-control already has live workflow bundles that package a ticket, peer admission, authority grant, and supporting receipts. The same handoff shape is now needed by remote dataspace traffic, job workers, retention clearance workflows, and artifact sync. Keeping each subsystem with a bespoke bundle creates repeated UX, repeated validation, and uneven diagnostics. Molten needs one reusable peer handoff bundle that carries bootstrap/session evidence and optional operation grants without making the bundle itself authority.

## What Changes

- Define a generic `peer-handoff-bundle-v1` with ticket/session/admission evidence, negotiated scopes, capability/resource/policy refs, optional authority grants, freshness, and supporting receipts.
- Add verify, gate, import, apply, and diagnose flows that validate member refs, expected peer/node/topic/scope bindings, freshness, and policy-resource evidence before state-root import.
- Make node-control live workflow bundles a specialization or compatibility wrapper over the generic handoff model.
- Let remote dataspace, job worker, retention-clearance, and remote artifact sync flows consume the same handoff evidence when the declared scope matches.

## Impact

- **Files**: peer handoff core, node-control bundle compatibility, subsystem handoff consumers, CLI commands, catalog summaries, and tests.
- **Testing**: positive handoff import/apply tests and negative missing-member, wrong-scope, stale-ticket, wrong-peer, wrong-topic, malformed-authority, and transport-only evidence tests.
- **Security**: handoff bundles remain operational evidence. Authority, policy, resource, provenance, source-gate, retention, and execution trust still require subsystem-specific gates.
