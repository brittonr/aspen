# Job DAG Delta: Capability-backed Admission

### Requirement: Job admission capabilities resolve to authority contexts
r[molten.job_dag_capability_admission.spec.context_resolution] Job admission MUST treat capability refs as canonical `authority-context-v1` refs available in the target registry and MUST deny refs that do not resolve to authority contexts.

#### Scenario: Placeholder capability ref denies
- GIVEN a synced target job closure
- AND a job admission request with a capability ref that is not an authority context in the target registry
- WHEN admission runs
- THEN the decision is deny
- AND the receipt includes a `capability-authority-context` failure check
- AND no stage logic is executed

### Requirement: Authority context must admit job execution scope
r[molten.job_dag_capability_admission.spec.job_execute_scope] Job admission MUST call the authority admission path for capability `job:execute` scoped to the job ref and require at least one passing authority admission.

#### Scenario: Matching authority context admits
- GIVEN a target registry containing an authority context with capability `job:execute` scoped to the job ref
- AND the synced closure and resource checks pass
- WHEN admission runs
- THEN the decision is pass
- AND the job admission plan and receipt bind the authority admission receipt ref

#### Scenario: Wrong scope denies
- GIVEN a target authority context with capability `job:execute` scoped to a different job ref
- WHEN admission runs for this job
- THEN the decision is deny
- AND no execution authority is minted

### Requirement: Synced artifact possession is not authority
r[molten.job_dag_capability_admission.spec.no_artifact_authority] Presence of synced job/stage artifacts MUST NOT grant execution authority without a target authority context that admits `job:execute`.

#### Scenario: Sync succeeded but authority context missing
- GIVEN sync-loopback has installed the full job/stage closure
- AND no target authority context admits the job
- WHEN admission runs
- THEN admission denies
- AND no stage output or execution receipt is produced
