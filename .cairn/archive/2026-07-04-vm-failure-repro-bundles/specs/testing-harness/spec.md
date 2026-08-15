## ADDED Requirements

### Requirement: VM failures export sealed diagnostic repro bundles
r[molten.testing.multinode.vm_failure_repro_export] Molten SHOULD export sealed diagnostic failure repro bundles for VM multinode shard or aggregate failures, binding scenario, topology, node evidence, child receipts, validation receipts, diagnostic logs, replay status, redaction policy, and evidence-only caveats.

#### Scenario: Denied VM shard produces diagnostic bundle
- GIVEN a VM shard with denied, unavailable, or failed validation evidence
- WHEN failure repro export runs
- THEN the bundle binds the scenario fixture ref, topology ref, node summary refs, child receipt refs, diagnostic log refs, validation refs, replay status, and caveats
- AND the bundle is marked diagnostic-only.

#### Scenario: VM live observation is not replayable by default
- GIVEN a VM failure bundle containing unrecorded live transport observations
- WHEN the bundle is verified
- THEN verification records non-replayable diagnostic status
- AND the bundle cannot satisfy deterministic pass evidence.

### Requirement: VM failure repro bundles fail closed on privacy, tamper, and pass-gate use
r[molten.testing.multinode.vm_failure_repro_privacy_gate] Molten MUST reject VM failure repro bundles that are tampered, unsealed, stale, private without matching reveal receipts, missing redaction policy, or presented as pass evidence.

#### Scenario: Tampered VM failure bundle is rejected
- GIVEN a sealed VM failure repro bundle whose topology, node summary, child receipt, diagnostic ref, or seal metadata has been modified
- WHEN verification runs
- THEN verification denies before materializing bundle contents
- AND diagnostics identify the stale or tampered binding.

#### Scenario: Diagnostic bundle cannot pass gate
- GIVEN a verified VM failure repro bundle
- WHEN a pass evidence gate evaluates it
- THEN the gate rejects it as diagnostic-only evidence
- AND no diagnostic log can override the canonical failure decision.
