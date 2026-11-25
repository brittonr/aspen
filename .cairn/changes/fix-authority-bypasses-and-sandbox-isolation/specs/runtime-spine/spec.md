# Runtime spine deltas

## ADDED Requirements

### Requirement: Envelopes, membership commits, context profiles, DAG sync, generic transport shells, SpaceWasm, and node-host storage fail closed on bypasses
r[molten.runtime_spine.admission_and_capability_hardening] `Envelope` deserialization MUST enforce checked parsers for `ActorId`, `Capability`, and `EvidenceRef`; `raft::membership::commit` MUST bind preflight receipts to the exact request; generic registered transport shells MUST prevent request replay; `context-profile` `merge_refs` MUST reject conflicting `retention_refs`; `dag_sync` MUST refresh authority/resources per transfer; `spacewasm` MUST bind `policy_json` digest to `authority_ref` and reject `/` in actor names; and node-host filesystem capabilities MUST isolate stage cleanup per writer and prevent symlink/mode TOCTOU.

#### Scenario: Unvalidated Envelope token or mismatched membership preflight denies
- GIVEN a serialized `Envelope` with an empty/oversized `ActorId` or a `raft::membership::commit` reusing an unrelated passing preflight receipt
- WHEN admission or deserialization runs
- THEN it MUST fail closed with a typed error.

#### Scenario: Conflicting retention override or replayed generic transport request denies
- GIVEN a conflicting `retention_refs` override in `context-profile` or a second execution of the same `request_ref` in generic `fabric_transport/shell.rs`
- WHEN the operation executes
- THEN it MUST deny rather than passing or replaying.
