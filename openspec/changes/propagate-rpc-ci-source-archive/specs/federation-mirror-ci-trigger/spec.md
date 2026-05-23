## MODIFIED Requirements

### Requirement: Regression test for CiTriggerPipeline on tree-walked config

The test suite SHALL include a test that exercises the full `handle_trigger_pipeline` path with a real `ForgeNode` containing a commit tree with `.aspen/ci.ncl`, verifying that the tree walk, Nickel parse, source archive creation, and pipeline creation succeed.

#### Scenario: Integration test with real tree walk

- **WHEN** a test creates a `ForgeNode`, imports a commit tree containing `.aspen/ci.ncl` with valid Nickel config, and calls `handle_trigger_pipeline`
- **THEN** the response SHALL have `is_success: true`
- **AND** the response SHALL contain a non-empty `run_id`

#### Scenario: RPC trigger regression captures source hash [r[federation-mirror-ci-trigger.rpc-trigger-source-hash-regression]]

- GIVEN a test invokes `handle_trigger_pipeline` with an orchestrator that has a blob store
- AND the checked-out Forge tree contains a root `flake.nix`
- WHEN the handler starts the pipeline
- THEN the resulting run context MUST include a non-empty `source_hash`
- AND the source archive referenced by that hash MUST materialize a root directory containing `flake.nix`
