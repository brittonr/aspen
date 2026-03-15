# Deploy protocol contracts
# Auto-generated Nickel contracts. Do not edit manually.

# Request to initiate a deployment.
let DeployRequest = {
  # Artifact reference (store path or blob hash).
  artifact | String,
  # Binary to validate inside a Nix store path. `None` → default `bin/aspen-node`.
  expected_binary | String | optional,
  # Health check timeout in seconds.
  health_timeout_secs | Number,
  # Max concurrent node upgrades.
  max_concurrent | Number,
  # Whether to track deployment lifecycle state in Raft KV.
  #
  # When `true`, the executor writes state under `_deploy:state:{deploy_id}:` including metadata, per-node status, and rollback points. When `false`, only CI job logs are persisted (stateless push deploy).
  stateful | Bool | default = true,
  # Deployment strategy (e.g., "rolling").
  strategy | String,
  # When `true`, only validate the artifact exists and the expected binary is present. Skip profile switch and process restart. Use this for CI pipeline tests that verify the deploy stage resolves artifacts without modifying the running cluster.
  validate_only | Bool | default = false,
}in

# Result of initiating a deployment.
let DeployInitResult = {
  # Deployment ID assigned by the coordinator.
  deploy_id | String | optional,
  # Error message if rejected.
  error | String | optional,
  # Whether the deployment was accepted.
  is_accepted | Bool,
}in

# Status of an in-progress deployment.
let DeployStatusResult = {
  # Elapsed milliseconds since deployment started.
  elapsed_ms | Number | optional,
  # Error message if applicable.
  error | String | optional,
  # Whether a deployment was found.
  is_found | Bool,
  # Per-node statuses.
  nodes | Array DeployNodeStatus,
  # Overall status string (pending, deploying, completed, failed, rolled_back).
  overall_status | String | optional,
}in

# Per-node deployment status entry.
let DeployNodeStatus = {
  # Error if node failed.
  error | String | optional,
  # Node ID.
  node_id | Number,
  # Status string (pending, draining, upgrading, healthy, failed).
  status | String,
}in

{
  DeployRequest,
  DeployInitResult,
  DeployStatusResult,
  DeployNodeStatus,
}
