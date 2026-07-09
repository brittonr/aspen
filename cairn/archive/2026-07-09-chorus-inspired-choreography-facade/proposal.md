## Why

ChoRus is a useful Rust reference for choreographic-programming ergonomics: typed locations, located values, an EPP-as-dependency-injection operator interface, runner/projector separation, and pluggable transports. Molten already has the stronger semantic boundary: protocol manifests lower to Trellis global choreography, pass Trellis projectability, project to local endpoints, and execute through dataspace-backed interpreters with canonical Preserves receipts.

Molten should adapt the ergonomic ideas that make choreographies easier to author and test, while keeping Trellis, canonical Preserves, BLAKE3 refs, authority/policy/resource gates, and protocol-session receipts as the normative behavior.

## What Changes

- Define ChoRus as a non-normative design reference for typed choreography ergonomics only.
- Add a typed Rust facade direction generated from admitted protocol manifests and Trellis-projected endpoints.
- Shape the facade around an EPP-as-DI style operator interface that is a deterministic Sans-IO transition core, not a transport or runtime dependency.
- Preserve role-scoped payload access and runner/projector parity without bypassing Molten protocol messages, dataspace state, or evidence gates.
- Explicitly reject adopting ChoRus transports, serde_json message identity, runtime projection semantics, or compatibility claims.
- Require positive and negative fixtures for projectable manifests, wrong-role access, wrong labels/payloads, rejected manifests, missing evidence, and no-ChoRus dependency checks.

## Impact

- **Files**: `cairn/specs/choreography/spec.md`, protocol facade docs, future generated facade modules, protocol-session tests, and README references.
- **Testing**: positive fixtures for generated typed facades and runner/projection parity; negative fixtures for non-projectable manifests, wrong role/label/payload access, missing authority/policy/resource evidence, attempts to use ChoRus transports, and dependency drift.
- **Security**: ChoRus remains external prior art only. It grants no authority, transport trust, provenance, policy admission, replay evidence, or semantic compatibility. Molten's canonical protocol records and receipts remain the boundary.
