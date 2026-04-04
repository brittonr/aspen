## 1. Zero-Test Crate Coverage

- [x] 1.1 Add unit tests for `aspen-secrets-handler`: test secret encryption, decryption, key rotation, and access control RPC handlers
- [x] 1.2 Add integration tests for `aspen-snix-bridge`: test gRPC bridge startup, nix-daemon protocol handling, and connection lifecycle

## 2. Mutation Testing Infrastructure

- [ ] 2.1 Add `cargo-mutants` to the nix devshell and document usage in AGENTS.md
- [ ] 2.2 Run initial `cargo mutants -p aspen-coordination --timeout 60` and record baseline mutation score
- [ ] 2.3 Run initial `cargo mutants -p aspen-core --timeout 60` and record baseline mutation score
- [ ] 2.4 Write tests to kill surviving mutants in `aspen-coordination/src/verified/` functions
- [ ] 2.5 Write tests to kill surviving mutants in `aspen-core/src/verified/` functions
- [ ] 2.6 Add a `mutants` nextest profile and nix app (`nix run .#mutants`) for running mutation tests
- [ ] 2.7 Add nightly CI job configuration for scheduled mutation testing runs

## 3. Proptest Model-Based Tests (Coordination)

- [x] 3.1 Expand `aspen-coordination/tests/proptest_model_based.rs` with lock model that tracks holder, fencing token, and expiry across arbitrary acquire/release/renew/expire sequences
- [x] 3.2 Add proptest model-based tests for distributed queues: enqueue/dequeue/ack sequences verified against an in-memory FIFO model
- [x] 3.3 Add proptest model-based tests for distributed barriers: participant join/wait/release verified against threshold model
- [x] 3.4 Add proptest model-based tests for leader elections: concurrent candidacy verified for at-most-one-leader-per-term invariant
- [x] 3.5 Add proptest model-based tests for rate limiters: request sequences verified against token bucket model

## 4. Madsim Consensus Scenarios

- [x] 4.1 Add madsim test: split-brain partition of 5-node cluster into {1,2} and {3,4,5}, writes on both sides, partition heal, verify convergence and no data loss
- [x] 4.2 Add madsim test: slow follower with 100x AppendEntries latency, verify cluster availability and eventual catch-up
- [ ] 4.3 Add madsim test: snapshot triggered during add-learner, verify learner receives consistent state
- [ ] 4.4 Add madsim test: membership change proposed during snapshot transfer, verify both complete correctly
- [ ] 4.5 Add madsim test: clock-skewed TTL where nodes have divergent clocks, verify lock expiry and lease renewal use leader's clock

## 5. Fault Injection Points

- [ ] 5.1 Add buggify fault injection point in redb Raft log append path (simulate I/O error on write)
- [ ] 5.2 Add buggify fault injection point in redb snapshot persist path (simulate partial fsync)
- [ ] 5.3 Add buggify fault injection point in Iroh connection establishment (simulate timeout on connect)
- [ ] 5.4 Add buggify fault injection point in Iroh stream creation (simulate stream reset during snapshot transfer)
- [ ] 5.5 Add buggify fault injection point in blob download path (simulate truncated read, hash mismatch)
- [ ] 5.6 Write madsim tests exercising each new fault injection point to verify graceful error handling

## 6. Serialization Snapshot Tests

- [x] 6.1 Add `insta` as a dev-dependency to `aspen-client-api`, `aspen-cli`, and `aspen-core`
- [x] 6.2 Write insta snapshot tests for all RPC request/response message serialization in `aspen-client-api`
- [ ] 6.3 Write insta snapshot tests for CLI output formatting (`cluster status`, `kv get`, `kv scan`) in `aspen-cli`
- [ ] 6.4 Write insta snapshot tests for snafu error chain Display output in `aspen-core` and `aspen-raft`
- [ ] 6.5 Add insta redaction rules for timestamps, node IDs, and UUIDs in snapshot tests

## 7. Under-Tested Handler Crates

- [ ] 7.1 Add integration tests for `aspen-forge-handler` using patchbay harness: test repo create, push, clone, ref listing RPCs
- [ ] 7.2 Add integration tests for `aspen-ci-handler` using patchbay harness: test pipeline trigger, status query, log retrieval RPCs
- [ ] 7.3 Add integration tests for `aspen-docs-handler`: test document create, sync, and merge RPCs

## 8. CI and Nextest Configuration

- [ ] 8.1 Add new madsim tests to appropriate nextest profile overrides with extended timeouts
- [ ] 8.2 Add `cargo insta test` step to the standard CI pipeline
- [ ] 8.3 Document the mutation testing workflow and `cargo insta review` in AGENTS.md
