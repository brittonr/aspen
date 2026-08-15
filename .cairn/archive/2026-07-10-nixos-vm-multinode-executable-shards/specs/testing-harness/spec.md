## ADDED Requirements

### Requirement: NixOS VM shard checks are executable derivations
r[molten.testing.nixos_vm_multinode.executable_shard_derivations] Molten MUST expose named executable NixOS VM checks or apps for smoke, live-control, service/job coordination, restart recovery, VM fault evidence, and full aggregation; each shard MUST preserve canonical shard evidence and MUST NOT depend on log text for its pass decision.

#### Scenario: Individual shard emits canonical evidence
- GIVEN a requested VM shard check for smoke, live-control, service/job, restart, or fault evidence
- WHEN the shard completes on a supported host
- THEN its realized output contains a canonical `nixos-vm-shard-run-v1` receipt, required child receipts, diagnostic log refs, and evidence-only caveats
- AND log output is not accepted as a substitute for missing child receipts.

#### Scenario: Unsupported platform remains non-pass evidence
- GIVEN the host cannot execute the requested VM shard support boundary
- WHEN the shard check runs
- THEN it emits deny or unavailable evidence according to the declared unavailable policy
- AND unavailable execution is not counted as pass evidence for that shard.

### Requirement: VM aggregate consumes child shard outputs
r[molten.testing.nixos_vm_multinode.executable_shard_aggregate] Molten MUST build the full multinode VM aggregate from declared child shard outputs and reject missing, denied, stale, unavailable-as-pass, or log-only child evidence before accepting aggregate pass evidence.

#### Scenario: Aggregate indexes passing shards
- GIVEN passing shard outputs for every required VM shard in the declared matrix
- WHEN the aggregate check evaluates those outputs
- THEN it emits a canonical aggregate receipt binding each child shard ref, topology ref, package ref, manifest ref, and evidence-only caveat
- AND reviewers can inspect each shard independently.

#### Scenario: Missing child shard denies aggregate pass
- GIVEN a required shard output is absent, denied, stale, or represented only by logs
- WHEN aggregate validation runs
- THEN the aggregate decision is deny or unavailable
- AND diagnostics name the invalid child shard class.
