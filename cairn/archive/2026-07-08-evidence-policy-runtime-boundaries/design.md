## Context

The architecture document already names separate conceptual layers: core, policy/evidence, runtime, and adapters. A future crate layout needs those concepts expressed as source ownership and dependency direction before implementation moves code across crate boundaries.

## Design

### Evidence ownership

Evidence modules own canonical receipt values, receipt parsing, chain verification inputs, provenance summaries, and evidence-only boundary statements. They should not perform side effects or make policy authority decisions by themselves.

### Policy ownership

Policy modules own deterministic admission decisions over explicit inputs: authority context, capability refs, resource grants, provenance, retention evidence, and reviewed policy refs. Policy may consume evidence summaries, but it should not require runtime shells or adapter availability.

### Runtime ownership

Runtime modules own turn progression, dataspace/vat/service state machines, and execution planning. Runtime should consume admitted policy/evidence results and return planned effects rather than directly mutating adapters.

### Adapter ownership

Adapters own filesystem, transport, executor, store, and environment effects. They must not grant trust from availability or transport identity alone and must record evidence for outcomes that matter to replay.

### Staged extraction

The first step can be source organization and dependency checks inside the root crate. Crate extraction should occur after boundaries are clean enough to preserve compatibility re-exports and focused validation.

## Non-goals

- Do not assert behavioral correctness from evidence linkage alone.
- Do not make policy depend on live adapter availability.
- Do not extract every layer into crates in this package unless the boundary checks already prove readiness.
