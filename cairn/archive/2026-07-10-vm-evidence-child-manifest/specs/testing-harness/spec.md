## ADDED Requirements

### Requirement: VM evidence manifest includes child receipt closure
r[molten.testing.vm_evidence.child_artifact_manifest_completeness] Molten MUST preserve a VM evidence manifest that includes every canonical child receipt and diagnostic log referenced by VM test-run, prod-soak, shard, aggregate, validation, live-control, service/job, coordination, and VM fault evidence.

#### Scenario: Manifest contains all referenced child receipts
- GIVEN a VM run with test-run, prod-soak, shard, aggregate, live-control, service/job, coordination, and fault validation child refs
- WHEN the VM evidence manifest is emitted
- THEN the manifest includes each referenced canonical artifact with path, artifact kind, content ref, diagnostic-only flag, and caveats
- AND diagnostic logs are marked as diagnostic-only.

#### Scenario: Omitted child receipt is visible
- GIVEN a VM run whose top-level receipt references a child artifact not listed in the manifest
- WHEN manifest closure validation runs
- THEN validation denies or records unavailable evidence before accepting the manifest as complete
- AND diagnostics name the missing child ref.

### Requirement: VM evidence manifest closure fails closed
r[molten.testing.vm_evidence.manifest_reference_closure] Molten MUST reject VM evidence-manifest pass claims when required child artifacts are missing, duplicated, content-ref mismatched, wrong-kind, unreferenced, or represented only by logs.

#### Scenario: Tampered manifest entry denies
- GIVEN a manifest entry whose stored path or content no longer matches its recorded content ref
- WHEN manifest closure validation evaluates the evidence
- THEN validation denies before pass evidence is accepted
- AND diagnostics name the mismatched path or ref.

#### Scenario: Log-only child cannot satisfy closure
- GIVEN a manifest that includes diagnostic logs for a child workflow but omits the canonical child receipt
- WHEN closure validation runs
- THEN validation rejects the manifest as incomplete
- AND logs remain diagnostic-only evidence.
