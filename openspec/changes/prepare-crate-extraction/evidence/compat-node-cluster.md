# Node and Cluster Compatibility Evidence
Generated: 2026-04-25T02:20:11Z

## cargo check -p aspen --no-default-features --features node-runtime
    Checking aspen-rpc-handlers v0.1.0 (/home/brittonr/git/aspen/crates/aspen-rpc-handlers)
    Checking aspen v0.1.0 (/home/brittonr/git/aspen)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 8.78s

## cargo check -p aspen-cluster
    Checking aspen-raft v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft)
    Checking aspen-cluster v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.26s

## cargo check -p aspen-rpc-handlers
    Checking aspen-cluster-handler v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-handler)
    Checking aspen-rpc-handlers v0.1.0 (/home/brittonr/git/aspen/crates/aspen-rpc-handlers)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.65s

