# Plugin Host Delta: Plugin extension domain contract hardening

### Requirement: Plugin extension authoring contracts enforce domain invariants
r[molten.plugin_extension_contracts.domain_hardening.authoring_contracts] Plugin extension Nickel authoring contracts MUST reject malformed BLAKE3 refs, empty required evidence arrays, invalid extension ids, invalid versions, invalid profiles, invalid replay classes, missing conformance refs, and duplicate hostcall descriptor identities before reviewed Preserves exports are refreshed.

#### Scenario: Valid extension contract exports
- GIVEN a plugin extension contract whose ids, refs, profile, conformance refs, hostcall descriptors, and evidence refs satisfy the reviewed domain contracts
- WHEN the contract is evaluated through Nickel for export
- THEN the export succeeds and preserves the reviewed hostcall descriptor bindings

#### Scenario: Malformed extension contract fails early
- GIVEN a plugin extension contract with a malformed ref, unsupported profile, empty required refs, invalid extension id, or duplicate hostcall descriptor identity
- WHEN the contract is evaluated through Nickel
- THEN export fails before the malformed contract can be checked in as Preserves evidence

### Requirement: Plugin capability grants enforce attenuation and proof invariants
r[molten.plugin_extension_contracts.domain_hardening.grant_invariants] Plugin capability grant Nickel contracts MUST require proof refs, policy refs, resource refs, effect manifest refs, effect receipt refs, issuer refs, coherent attenuation depth/window values, and revocation evidence when a grant is marked revoked.

#### Scenario: Valid capability grant exports
- GIVEN a plugin capability grant with matching subject refs, proof evidence, non-empty resource/effect/policy refs, and a coherent attenuation window
- WHEN the grant is evaluated through Nickel for export
- THEN export succeeds and the resulting grant remains suitable for Rust admission validation

#### Scenario: Incoherent capability grant fails early
- GIVEN a plugin capability grant with empty proof refs, inverted validity turns, over-delegation, malformed refs, or a revoked flag without revocation evidence
- WHEN the grant is evaluated through Nickel
- THEN export fails before the grant can be bound into hostcall authority evidence
