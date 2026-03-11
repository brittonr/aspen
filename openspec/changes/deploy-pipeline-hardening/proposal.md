## Why

The deploy pipeline sends real RPCs over iroh QUIC as of the previous change, but several gaps remain that make it unsafe for production. Nodes restart without draining in-flight client RPCs, blob-based upgrades can't fetch the binary, leader failover abandons in-progress deployments, the CI handler still stubs its RPCs, and health checks don't verify Raft replication catchup. Each of these is a production failure mode that needs closing before the pipeline can be trusted with real clusters.

## What Changes

- Integrate DrainState into the client RPC hot path so in-flight requests complete before a node upgrade restarts the process. New RPCs during drain receive NOT_LEADER so clients fail over.
- Replace PlaceholderNodeRpcClient in aspen-ci-handler with the real IrohNodeRpcClient so CI-triggered deploys reach target nodes. Extract the shared client to avoid duplication.
- Wire iroh-blobs download into handle_node_upgrade so blob-based deployments fetch the binary to the staging directory before the executor tries to swap it.
- Call DeploymentCoordinator::check_and_resume() on leader election so a new leader finalizes in-progress deployments left behind when the previous leader upgraded itself.
- Deepen the deploy health check to verify Raft replication progress (log gap) in addition to the node's self-reported status, preventing premature healthy marking of nodes still catching up.

## Capabilities

### New Capabilities

- `deploy-drain`: Graceful drain integration between the upgrade executor and the client RPC handler, ensuring in-flight operations complete before process restart.
- `deploy-blob-fetch`: Blob artifact download via iroh-blobs during node upgrade, bridging the gap between the coordinator's blob hash reference and the executor's expectation of a staged binary.
- `deploy-leader-resume`: Automatic resumption of in-progress deployments after leader failover, driven by check_and_resume() on leader election.
- `deploy-health-depth`: Raft-aware health checking that verifies replication log gap before marking an upgraded node healthy.

### Modified Capabilities

- `ci`: CI deploy dispatcher switches from stub RPCs to real iroh QUIC RPCs, sharing the IrohNodeRpcClient with the cluster handler.

## Impact

- aspen-rpc-handlers: ClientProtocolHandler gains drain-awareness (check DrainState before dispatch)
- aspen-rpc-core: ClientProtocolContext gains optional Arc<DrainState> field
- aspen-cluster-handler: handle_node_upgrade passes shared DrainState to executor, gains blob fetch
- aspen-ci-handler: RpcDeployDispatcher takes endpoint, PlaceholderNodeRpcClient removed
- aspen-deploy: IrohNodeRpcClient extracted to shared location, health check gains log gap logic
- aspen-cluster: bootstrap wires DrainState into context, leader election calls check_and_resume()
- aspen-constants: new MAX_HEALTHY_LOG_GAP constant
