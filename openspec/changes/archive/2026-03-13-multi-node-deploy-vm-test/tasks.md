## 1. VM Test Scaffolding

- [x] 1.1 Create `nix/tests/ci-dogfood-deploy-multinode.nix` with 3-node NixOS VM test structure: deterministic iroh keys, shared cookie, `mkNodeConfig` helper, node1 with `enableCi = true` and `enableWorkers = true`
- [x] 1.2 Add WASM plugin setup (KV + Forge) using `lib/wasm-plugins.nix` helper, install on all 3 nodes
- [x] 1.3 Wire test into `flake.nix` as `checks.x86_64-linux.ci-dogfood-deploy-multinode-test`

## 2. Cluster Formation

- [x] 2.1 Write Python test helpers: `get_ticket()`, `cli()`, `cli_text()`, `get_endpoint_addr_json()`, `wait_for_healthy()` — adapted from `multi-node-cluster.nix` for 3-node topology
- [x] 2.2 Write cluster bootstrap sequence: `cluster init` on node1, `add-learner` for nodes 2 and 3, `change-membership 1 2 3`, verify 3 voters via `cluster status`

## 3. Forge + CI Pipeline Setup

- [x] 3.1 Create Forge repo, enable CI auto-trigger, push a cowsay Nix flake with `.aspen/ci.ncl` containing build + deploy stages (reuse pattern from `ci-dogfood-deploy.nix`)
- [x] 3.2 Poll pipeline status until build stage completes, verify `succeeded`

## 4. Deploy Stage Validation

- [x] 4.1 Write a sentinel KV key (`deploy-test-key`) before the deploy pipeline starts
- [x] 4.2 Wait for pipeline to reach terminal status, extract deploy stage result from pipeline JSON
- [x] 4.3 Verify deploy stage executed (check deployment was created, artifact was resolved from build job)
- [x] 4.4 Check deployment targeted all 3 nodes (read `_sys:deploy:*` KV keys or parse deploy logs)

## 5. Post-Deploy Health Checks

- [x] 5.1 Verify `cluster health` succeeds after pipeline completes
- [x] 5.2 Read back sentinel KV key and verify value matches what was written
- [x] 5.3 Add final assertion message: "MULTI-NODE DEPLOY DOGFOOD PASSED"

## 6. Build and Validate

- [x] 6.1 Run `nix build .#checks.x86_64-linux.ci-dogfood-deploy-multinode-test --impure --option sandbox false` and verify it passes
- [x] 6.2 Review test logs for deploy ordering (followers before leader) and any unexpected errors
