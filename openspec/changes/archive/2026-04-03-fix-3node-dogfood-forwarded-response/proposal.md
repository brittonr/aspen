## Why

3-node dogfood fails because forwarded RPC responses get truncated under QUIC load. After a 34K-object git push, the Raft log hits 600K+ entries. Snapshot transfers to followers saturate QUIC bandwidth, causing `send_rpc_on_stream` responses to be cut short. Postcard deserialization fails with "Hit the end of buffer" — workers can't dequeue jobs, workflows stall, and the pipeline reports false success.

Three interrelated bugs surface:

1. Forwarded responses lack length-prefix framing, so truncated streams produce silent deserialization failures instead of clear errors
2. RefUpdate announcements processed by multiple nodes trigger duplicate pipeline workflows
3. Pipeline status sync reads stale/corrupt workflow state, reporting Success for incomplete pipelines

## What Changes

- Add length-prefix framing to forwarded RPC responses in `IrohWriteForwarder::send_rpc_on_stream` so truncation is detected as a framing error rather than a deserialization error, and partial reads are retried
- Deduplicate CI pipeline triggers so the same commit+repo pair only creates one pipeline run
- Validate pipeline status against actual stage/job completion before reporting Success

## Capabilities

### New Capabilities

- `length-framed-forwarding`: Length-prefix framing on forwarded RPC streams to detect truncation and enable retry
- `ci-pipeline-dedup`: Deduplicate CI pipeline triggers for the same commit+repo pair
- `pipeline-status-validation`: Validate pipeline completion status against actual stage/job state

### Modified Capabilities

## Impact

- `crates/aspen-raft/src/iroh_write_forwarder.rs` — framing changes to `send_rpc_on_stream`
- `crates/aspen-rpc-handlers/` or `crates/aspen-ci/` — protocol handler response framing must match
- `crates/aspen-ci/src/orchestrator/` — pipeline trigger dedup and status validation
- Wire protocol change: length prefix is additive but existing clients sending unframed responses need handling during rolling upgrades
