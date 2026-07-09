## ADDED Requirements

### Requirement: Node lifecycle accepts profile-backed configuration
r[molten.node_runtime.profile_backed_config] Molten MUST allow node lifecycle commands to construct durable `node-config-v1` from a checked exported node profile or profile evidence ref. Runtime startup MUST consume checked profile artifacts and MUST NOT evaluate Nickel during node init, run, serve, or startup receipt generation.

#### Scenario: Profile-backed node config is written
- GIVEN a checked exported node profile with supported metadata, state layout, required adapter profiles, source-gate refs, policy refs, capability refs, resource refs, and effect-profile refs
- WHEN `molten node init` is run with that profile
- THEN the durable node config records the profile-selected values
- AND the config ref is derived from canonical Preserves bytes.

#### Scenario: Runtime Nickel evaluation is denied
- GIVEN a node startup command receives a Nickel source file instead of a checked exported profile artifact or profile ref
- WHEN startup validation runs
- THEN startup denies before side effects that depend on the profile
- AND diagnostics state that runtime startup consumes checked exports rather than evaluating Nickel.

### Requirement: Startup receipts bind effective profile evidence
r[molten.node_runtime.profile_startup_receipt_binding] Node startup receipts MUST bind the effective profile ref, profile schema metadata, selected adapter profile refs, state-layout ref, source-gate refs, policy refs, capability refs, resource refs, effect-profile refs, and profile-resolution diagnostics.

#### Scenario: Startup binds matching profile metadata
- GIVEN a node starts from a checked exported profile whose content ref matches the supplied profile ref
- WHEN startup emits a receipt
- THEN the receipt records the matching profile ref, schema id, schema version, source language, profile identity, selected adapters, and evidence refs
- AND downstream review can distinguish profile evidence from authority, source-gate, adapter, and transport receipts.

#### Scenario: Tampered profile ref denies startup
- GIVEN a supplied profile ref does not match the checked profile artifact bytes
- WHEN node startup validates the profile evidence
- THEN startup denies before adapter start receipts are accepted
- AND diagnostics name the profile-ref mismatch.

### Requirement: Profile and CLI overrides are explicit
r[molten.node_runtime.profile_override_policy] Molten MUST apply profile-to-CLI override rules through a deterministic core that records every accepted override and denies overrides that weaken required profile invariants, omit required evidence, or change a release-tier profile into a local fixture configuration.

#### Scenario: Allowed local override is recorded
- GIVEN a development profile marks a non-security path field as overrideable
- WHEN an operator supplies the corresponding CLI override
- THEN profile resolution records the override source and effective value in diagnostics or receipts
- AND the canonical node config uses the effective value.

#### Scenario: Production invariant override denies
- GIVEN a production or release-tier profile requires a source-gate ref, required adapter, or resource-limit invariant
- WHEN an operator supplies a CLI override that removes or weakens that invariant
- THEN profile resolution denies before writing durable node config
- AND diagnostics identify the denied override and profile rule.

### Requirement: Local default node config is fixture-scoped
r[molten.node_runtime.local_default_config_caveat] Molten MAY retain current no-profile local node defaults for development fixtures, but startup receipts emitted from those defaults MUST carry a local-fixture caveat and MUST NOT satisfy production or release profile evidence requirements.

#### Scenario: No-profile init remains usable for fixtures
- GIVEN an operator runs `molten node init` without a profile in a local test state root
- WHEN durable config and startup receipts are emitted
- THEN the command preserves the local development behavior
- AND the receipts record that the config came from local fixture defaults.

#### Scenario: Fixture config cannot pass release profile gate
- GIVEN a release gate requires profile-backed configuration evidence
- WHEN the gate receives only a startup receipt produced from local defaults
- THEN the gate denies or records the config as insufficient for release evidence.
