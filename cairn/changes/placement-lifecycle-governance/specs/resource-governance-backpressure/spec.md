# Resource Governance Delta: placement lifecycle governance

### Requirement: Placement evaluates requests, limits, quotas, and capacity evidence
r[molten.placement.resource_requests_limits_quotas] Molten MUST gate placement decisions for actors, services, Wasm components, jobs, plugin hosts, workflow runners, and node-control operations through explicit resource requests, limits, quota refs, priority refs, capacity evidence refs, and assignment authority refs. Placement MUST deny when requested resources exceed admitted quota, limits are lower than requests, capacity evidence is missing or stale, or assignment authority is absent.

#### Scenario: Placement fits admitted capacity
- GIVEN a workload resource with requests, limits, quota refs, priority refs, target capacity evidence, and assignment authority
- WHEN the placement core evaluates the target
- THEN it emits a placement pass decision binding the workload, target, quota, capacity, and authority evidence.

#### Scenario: Over-quota placement denies
- GIVEN a workload resource whose requested capacity exceeds the admitted quota for its scope
- WHEN the placement core evaluates candidate targets
- THEN Molten denies placement
- AND diagnostics identify the quota ref and exceeded resource class.

### Requirement: Placement constraints, taints, and tolerations are explicit
r[molten.placement.constraint_profiles_taints_tolerations] Molten SHOULD support placement constraints, affinity and anti-affinity summaries, taints, tolerations, and defer preferences as explicit policy-bound inputs. Molten MUST deny placement claims that rely on unsupported selectors, hidden node assumptions, unsatisfied hard constraints, or missing tolerations for hard taints.

#### Scenario: Tolerated taint permits placement
- GIVEN a candidate target with a hard taint and a workload with a matching admitted toleration
- WHEN placement evaluates the target
- THEN the placement decision may pass while binding the taint and toleration evidence.

#### Scenario: Hidden node assumption denies
- GIVEN a workload that can be placed only if an unstated target property is true
- WHEN placement evaluates explicit target evidence and the property is absent
- THEN Molten denies or defers placement instead of assuming the property from logs or host state.
