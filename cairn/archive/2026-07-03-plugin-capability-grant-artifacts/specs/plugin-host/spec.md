## ADDED Requirements

### Requirement: Plugin capability grants are canonical artifacts
r[molten.plugin_capability_grants.grant_artifact] Molten MUST represent plugin host authority with canonical `plugin-capability-grant-v1` artifacts that bind the subject plugin ref, plugin id, active manifest ref, optional extension contract ref, hostcall descriptor ref, operation, input/output schema refs, resource refs and scope, effect manifest refs, policy refs, issuer/proof refs, attenuation metadata, revocation evidence refs, and replay/idempotency class. The BLAKE3 grant ref MUST identify the exact canonical grant value and MUST NOT be treated as authority unless the grant body parses and validates for the requested operation.

#### Scenario: Grant ref binds exact hostcall authority
- GIVEN a plugin manifest that declares `storage.read`
- AND a canonical capability grant bound to that manifest, operation, descriptor, schemas, resource, policy, effect, issuer, and proof evidence
- WHEN plugin hostcall admission evaluates the request
- THEN the hostcall may pass only by binding the matching capability grant ref
- AND a different BLAKE3 artifact ref is not accepted as authority.

### Requirement: Hostcall admission requires typed capability grants
r[molten.plugin_capability_grants.hostcall_admission] Plugin hostcall admission MUST require supplied capability grant refs to resolve to `plugin-capability-grant-v1` artifacts whose subject, manifest, extension contract, descriptor, operation, schemas, resources, effects, policies, and proofs match the selected hostcall descriptor before any Steel, Wasm, native-adapter, or remote-proxy host side effect can occur. Generic authority refs MAY be retained as compatibility or proof evidence, but they MUST NOT satisfy a descriptor that requires typed capability grants by themselves.

#### Scenario: Generic authority ref is insufficient
- GIVEN a plugin extension contract whose `storage.read` descriptor requires a typed capability grant
- WHEN a hostcall request supplies only a non-empty generic authority ref and no matching capability grant artifact
- THEN Molten emits a plugin hostcall receipt with decision `deny`
- AND diagnostics identify the missing typed capability grant.

### Requirement: Capability attenuation and revocation are deterministic
r[molten.plugin_capability_grants.revocation_attenuation] Plugin capability grant validation MUST enforce attenuation and revocation from explicitly supplied canonical evidence, including narrowed operations, resource sub-scopes, schema/profile constraints, delegation depth, budget refs, turn/tick validity evidence, and revocation receipt refs. The pure admission core MUST NOT read clocks, files, networks, or mutable revocation registries while deciding whether a grant is valid.

#### Scenario: Revoked grant denies hostcall
- GIVEN a plugin hostcall request with a capability grant whose operation and resource match the descriptor
- AND canonical revocation evidence invalidates that grant for the evaluated turn
- WHEN hostcall admission runs
- THEN the hostcall receipt decision is `deny`
- AND no host side effect is admitted.

### Requirement: Capability grants are Nickel-authored and canonically exported
r[molten.plugin_capability_grants.nickel_authoring] Human-authored plugin capability grant fixtures and grant templates SHOULD use typed Nickel contracts by default and MUST export checked-in canonical Preserves evidence before Rust validation consumes them. Runtime admission MUST NOT execute Nickel or treat Nickel source presence as authority.

#### Scenario: Invalid grant fixture fails before admission
- GIVEN a Nickel-authored plugin capability grant fixture whose resource ref does not match the declared hostcall descriptor
- WHEN export or validation checks run
- THEN the fixture fails before runtime admission can bind it
- AND the invalid Nickel source is not treated as trusted authority.
