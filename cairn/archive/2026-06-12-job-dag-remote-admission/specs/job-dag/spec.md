# Job DAG Delta: Remote Admission

### Requirement: Admission requests bind synced closure evidence and explicit authority
r[molten.job_dag_remote_admission.spec.request] Job admission requests MUST bind the job ref, sync evidence ref, selected stages, target peer, policy refs, capability refs, evidence refs, resource refs, and no-execution checks.

#### Scenario: Admission request is not authority by possession
- GIVEN a target registry containing synced job artifacts
- WHEN an admission request omits policy, capability, evidence, or resource refs
- THEN admission denies
- AND synced artifact possession does not grant execution authority
- AND no stage logic is executed

### Requirement: Admission plans verify target closure by canonical refs
r[molten.job_dag_remote_admission.spec.target_closure] Admission plans MUST verify the target registry contains the selected job/stage dependency closure by canonical artifact refs and MUST deny missing, tampered, or divergent closure members.

#### Scenario: Missing target dependency denies admission
- GIVEN a sync receipt for a job DAG
- AND a target registry missing one stage dependency artifact
- WHEN admission planning runs
- THEN the plan decision is deny
- AND the denial identifies the missing artifact ref
- AND no stage logic is executed

#### Scenario: Tampered target artifact denies admission
- GIVEN a target registry artifact at a claimed ref whose canonical envelope no longer matches the source/sync evidence
- WHEN admission planning runs
- THEN the plan decision is deny
- AND the receipt binds a tamper or ref-mismatch diagnostic

### Requirement: Admission topology uses Trellis DAG primitives
r[molten.job_dag_remote_admission.spec.trellis_topology] Admission planning MUST use Trellis topology/job-DAG primitives for selected-stage ordering, cycle rejection, unknown-stage rejection, and dependency satisfaction checks.

#### Scenario: Selected stage dependencies are unsatisfied
- GIVEN a job DAG where a selected stage depends on an unselected upstream stage that is not already materialized
- WHEN admission planning runs
- THEN the plan decision is deny
- AND the denial identifies unsatisfied selected-stage dependencies

### Requirement: Executable stages require artifact-backed admitted operations
r[molten.job_dag_remote_admission.spec.executable_artifact_gate] Executable job stages MUST be admitted only when their operation is represented by canonical artifact refs available at the target; raw closures, inline scripts, shell commands, host paths, mutable tags, and unverified URLs MUST deny.

#### Scenario: Raw closure stage denies before execution
- GIVEN a synced job DAG containing an executable stage with inline closure or shell/path config
- WHEN target admission runs
- THEN admission denies before execution
- AND the receipt includes a no-mobile-closures or executable-artifact-gate check

### Requirement: Admission checks target authority and resources before execution
r[molten.job_dag_remote_admission.spec.authority_resource] Admission MUST bind explicit target policy/capability/evidence/resource refs and deny stale, absent, or over-budget refs before any execution authority is minted.

#### Scenario: Resource budget is exceeded
- GIVEN a selected job stage whose profile exceeds the target resource refs
- WHEN admission runs
- THEN the plan and receipt decision is deny
- AND no executor is started

### Requirement: Admission receipts are canonical no-execution evidence
r[molten.job_dag_remote_admission.spec.receipts] Admission receipts MUST bind the request ref, job ref, sync ref, plan ref, closure refs, stage verdicts, resource verdict, decision, target peer, and checks including `no-execution`.

#### Scenario: Successful loopback admission remains non-executing
- GIVEN a target registry with a verified synced closure and explicit authority/resource refs
- WHEN loopback admission passes
- THEN the receipt decision is pass
- AND the receipt includes target-closure, topology, executable-artifact, authority, resource, and no-execution checks
- AND no stage output or execution receipt is produced
