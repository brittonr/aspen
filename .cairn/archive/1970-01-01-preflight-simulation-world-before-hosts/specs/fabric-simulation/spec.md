# Reference simulation manifest admission before host creation

## ADDED Requirements

### Requirement: Existing canonical simulated-world admission precedes reference hosts

r[molten.fabric_simulation.manifest_admission_before_host_creation] The public `run_reference_world` path MUST perform the existing canonical/core simulated-world admission before creating or activating reference extension hosts for any caller-provided manifest. It MUST use the same core authority and existing inclusive maximum of 64 nodes and 4,096 workload steps, without introducing a new count policy or duplicate preflight. For admitted manifests, reference profile/tier/service/host error and execution order MUST remain unchanged.

#### Scenario: The 65th node denies before reference hosts

- GIVEN a caller-built manifest with 65 distinct simulated nodes, including a reference service the host selector cannot recognize after otherwise valid reference nodes
- WHEN the public runner receives the manifest
- THEN the existing core node-count admission denies at 65/64 instead of returning the host selector error or constructing/activating a reference host.

#### Scenario: The 4,097th workload step denies before reference hosts

- GIVEN a caller-built manifest with 4,097 sequential workload steps, after otherwise valid reference nodes and with an unrecognized reference service in a node
- WHEN the public runner receives the manifest
- THEN the existing core workload-count admission denies at 4,097/4,096 instead of returning the host selector error or constructing/activating a reference host.

#### Scenario: Admitted manifests retain reference-host behavior

- GIVEN a manifest within existing core limits with a syntactically valid but unsupported reference service
- WHEN the public runner receives the manifest
- THEN core admission succeeds and the prior reference-service error is returned by host selection.

#### Scenario: Built-in reference simulation still executes

- GIVEN the existing built-in reference-world manifest
- WHEN the compiled `fabric-simulation run` CLI starts its reference simulation
- THEN the admitted simulation still completes and emits its normal run evidence.
