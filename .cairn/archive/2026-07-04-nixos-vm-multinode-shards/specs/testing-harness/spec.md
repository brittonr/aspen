## ADDED Requirements

### Requirement: NixOS multinode VM checks are sharded by scenario
r[molten.testing.nixos_vm_multinode.sharded_checks] Molten SHOULD expose named NixOS VM multinode shard checks whose scenario, command surface, required receipts, expected artifact kinds, unavailable policy, diagnostic logs, and caveats are declared before execution.

#### Scenario: Shard receipt binds the scenario boundary
- GIVEN a VM shard plan for smoke, live control, service/job coordination, restart recovery, or VM fault evidence
- WHEN the shard check completes
- THEN the shard receipt binds the scenario fixture ref, topology ref, node evidence refs, required child refs, diagnostic log refs, unavailable status, and evidence-only caveats
- AND the receipt states the shard evidence scope.

#### Scenario: Shard failure localizes the broken layer
- GIVEN a VM shard whose required canonical receipt is missing, denied, stale, or represented only by logs
- WHEN the shard receipt is generated
- THEN the shard decision denies or records unavailable according to the declared policy
- AND diagnostics name the missing or invalid receipt class.

### Requirement: NixOS multinode aggregate preserves child shard evidence
r[molten.testing.nixos_vm_multinode.shard_aggregate] Molten MUST treat the full multinode VM check as an aggregate over passing shard receipts and MUST NOT convert unavailable, skipped, denied, stale, or log-only child evidence into pass evidence.

#### Scenario: Aggregate binds every required shard
- GIVEN passing shard receipts for the declared VM shard matrix
- WHEN the aggregate VM evidence is emitted
- THEN the aggregate receipt binds each child shard ref, the topology ref, the package ref, and the manifest ref
- AND the aggregate remains review evidence over child receipts.

#### Scenario: Missing shard prevents aggregate pass
- GIVEN a full VM aggregate where a required shard receipt is absent or denied
- WHEN aggregate validation runs
- THEN the aggregate denies before pass evidence is accepted
- AND diagnostic logs cannot repair the missing child receipt.
