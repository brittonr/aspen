## ADDED Requirements

### Requirement: VM validation parses child receipt semantics
r[molten.testing.vm_evidence.child_receipt_semantic_validation] Molten MUST validate VM child workflow evidence by parsing canonical child receipt contents and checking expected topology, node ids, peer ids, operation ids, receipt classes, and pass or deny decisions rather than accepting child ref presence alone.

#### Scenario: Live-control child chain validates semantically
- GIVEN a VM test-run receipt with child refs for live-control send or receive, ingress, queue, dispatch, reconcile, ack, and protocol-gate evidence
- WHEN VM evidence validation evaluates the run against expected sender, receiver, peer, topic, and operation bindings
- THEN validation passes only if each child receipt class and semantic field matches the expected workflow
- AND transport or log evidence alone cannot replace missing authority, admission, queue, dispatch, reconcile, ack, or protocol refs.

#### Scenario: Wrong child receipt denies validation
- GIVEN a VM run whose child ref points to the wrong receipt class, wrong node, wrong peer, wrong operation id, denied decision, or stale topology
- WHEN semantic child validation runs
- THEN validation denies before accepting the VM pass claim
- AND diagnostics identify the mismatched child field.

### Requirement: VM validation gates expected child refs explicitly
r[molten.testing.vm_evidence.expected_child_ref_gate] Molten MUST require declared expected child refs or scenario-derived child expectations for VM pass evidence, and MUST reject missing, duplicate, log-only, stale, or undeclared child refs before accepting a cluster-testing claim.

#### Scenario: Expected child refs all bind
- GIVEN a VM validation request with explicit expected child refs for live-control, service/job, coordination, soak, and fault validation evidence
- WHEN the validator parses the VM test-run and child artifacts
- THEN each expected ref is present, classified, semantically checked, and bound into validation evidence
- AND extra diagnostic logs do not affect pass evidence.

#### Scenario: Missing expected child ref denies
- GIVEN a VM validation request that requires a child receipt ref not present in the test-run, manifest, or child artifact set
- WHEN validation runs
- THEN the decision is deny
- AND diagnostics name the missing expected child ref.
