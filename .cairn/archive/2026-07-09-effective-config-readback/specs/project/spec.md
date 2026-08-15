## ADDED Requirements

### Requirement: Effective config readback artifacts
r[molten.project.effective_config_readback.artifact] Molten SHOULD emit canonical effective-configuration readback artifacts that record schema metadata, normalized effective values, source traces, profile refs, override refs, default caveats, diagnostics, and checks. Effective-config artifact identity MUST be derived from canonical bytes using BLAKE3.

#### Scenario: Effective config has stable identity
- GIVEN the same checked profile inputs, CLI override inputs, and default policy
- WHEN Molten computes an effective-config readback twice
- THEN both readbacks have the same canonical BLAKE3 ref
- AND rendered text output is not used as the identity source.

#### Scenario: Hidden default is visible
- GIVEN an effective value comes from a local fixture default rather than a reviewed profile
- WHEN the readback artifact is emitted
- THEN the field source records the default origin and caveat
- AND release review can distinguish it from profile-backed configuration.

### Requirement: Config source traces are field-local
r[molten.project.effective_config_readback.source_trace] Each effective-config field SHOULD identify its source class, source ref or command input when available, override status, and caveats closely enough for reviewers to distinguish reviewed profile values from CLI overrides, environment-resolved shell inputs, ledger evidence, and fixture defaults.

#### Scenario: CLI override source is recorded
- GIVEN a profile value is overridden by an admitted CLI value
- WHEN effective-config readback runs
- THEN the field records both the profile source and the CLI override source
- AND diagnostics identify the override rule that admitted the value.

#### Scenario: Conflicting sources deny
- GIVEN two non-mergeable sources provide different values for a field that must be unique
- WHEN effective-config normalization runs
- THEN it denies with diagnostics naming the conflicting source classes and field.

### Requirement: Config validate, explain, diff, and fingerprint share a pure core
r[molten.project.effective_config_readback.cli_core] Molten SHOULD provide config validation/readback CLI commands whose decisions come from a deterministic pure core over explicit input records. The CLI shell MUST own filesystem reads, path resolution, environment lookup, artifact writing, and rendered diagnostics.

#### Scenario: Explain renders canonical readback
- GIVEN a valid effective-config input set
- WHEN an operator runs a config explain command
- THEN the command writes or references the canonical effective-config artifact
- AND the rendered explanation is a diagnostic view over that artifact.

#### Scenario: Diff uses normalized artifacts
- GIVEN two effective-config artifacts with different values or source traces
- WHEN an operator runs config diff
- THEN the diff is computed over normalized artifact fields
- AND diagnostics identify changed values, changed sources, and changed caveats.

### Requirement: Effective config readback is evidence-only
r[molten.project.effective_config_readback.evidence_only] Effective-config readback artifacts MUST NOT grant authority, policy admission, provenance trust, source-gate acceptance, resource rights, retention clearance, transport correctness, execution permission, or release eligibility by themselves.

#### Scenario: Readback cannot authorize mutation
- GIVEN a passing effective-config readback artifact
- WHEN a caller attempts to use it as the only evidence for install, run, delete, retention GC, live send, or policy mutation
- THEN the downstream gate denies unless the normal subsystem-specific receipts and authority are supplied independently.
