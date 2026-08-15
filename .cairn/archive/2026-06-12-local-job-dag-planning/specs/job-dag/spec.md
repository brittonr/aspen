# Job DAG Delta: Local Planning Profiles and Fusion Preview

### Requirement: Plans bind Trellis order and readiness
r[molten.local_job_dag_planning.spec.plan_trellis_binding] Local job plans MUST derive stage order and dependencies from the same Trellis-backed node-index/topology adapter used by execution.

#### Scenario: Plan stage order is Trellis-backed
- GIVEN a valid local job DAG
- WHEN a plan is emitted
- THEN the plan contains the Trellis-derived stage order
- AND each stage plan binds its Trellis index and dependency stage ids
- AND the plan checks include `trellis-topo-order` and `trellis-deps-ready`

### Requirement: Profiles are deterministic and side-effect-free
r[molten.local_job_dag_planning.spec.profile_determinism] Local job profiles MUST use deterministic canonical inputs only and MUST NOT depend on wall-clock time, system load, mtimes, network state, or stage execution side effects.

#### Scenario: Same DAG yields same profile ref
- GIVEN the same canonical job DAG and output request
- WHEN profiling is repeated with the same cache-index inputs
- THEN the profile ref is unchanged
- AND the profile checks include `no-wall-clock-time`

### Requirement: Fusion preview is conservative
r[molten.local_job_dag_planning.spec.fusion_safety] Fusion previews MUST admit only adjacent pure `map`/`filter` stages connected by stream edges, and MUST reject reduce, materialize, schema, effect, policy, or explicit materialization boundaries.

#### Scenario: Pure map/filter chain is previewed
- GIVEN adjacent `map`/`filter` stages with no schema, effect, policy, or materialization boundary
- WHEN fusion preview runs
- THEN the chain is included as preview-only evidence

#### Scenario: Boundary prevents fusion
- GIVEN adjacent stages separated by a schema ref, effect manifest, policy ref, reduce stage, materialize stage, or non-stream edge
- WHEN fusion preview runs
- THEN no fusion chain crosses that boundary

### Requirement: Planning receipts bind artifacts
r[molten.local_job_dag_planning.spec.receipts] Plan, profile, and fusion preview artifacts MUST have canonical receipts that bind the job ref, output request ref, artifact ref, diagnostics, and checks.

#### Scenario: Plan receipt binds plan ref
- GIVEN a plan artifact
- WHEN its receipt is emitted
- THEN the receipt artifact field equals the canonical plan ref
- AND the receipt decision is pass
