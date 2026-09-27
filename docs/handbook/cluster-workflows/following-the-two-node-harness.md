# Following the two-node harness

Mode: Walkthrough

This walkthrough follows the checked-in two-node manifest through the local process shell to its durable review directory. It is a source-checked path, not a transcript: no commands or runtime checks were executed for this article. Use the [technical diagnosis companion](../../technical/operations/receipt-first-cluster-diagnosis.md) for the evidence model; the practical question here is where each stage leaves something observable.

## 1. Identify the exact input

The [fixture](../../../tests/fixtures/cluster-harness/two-node.cluster) contains the header `molten.cluster.nodes.v1`, followed by `node:fixture-a` and `node:fixture-b`, in that order. There are no ports, topology links, fault schedules, or authority grants in this file. The [cluster planner](../../../src/parts/cluster/p000/body.rs) retains the node order and derives filesystem components `fixture-a` and `fixture-b` by removing the `node:` prefix.

The manifest parser checks its header and requires nodes. Planning rejects duplicate identities and unsafe node components. These boundaries matter before interpreting any child receipt: two labels in terminal output are not a substitute for the fixture-bound plan.

## 2. Choose separate, unused roots

Prerequisites are a source checkout with its Rust build dependencies available and permission to create two fresh sibling directories. The state root holds mutable node state; the run directory holds exported evidence. Neither may equal or contain the other. Existing directories are rejected by the ordinary invocation. Choose new paths rather than replacing a prior run.

The following command is source-checked, not executed. Its spelling follows the [CLI declaration and handler](../../../src/cli/ops/parts/cluster/p000/body.rs), reached through the [main module alias](../../../src/main.rs); the [CLI scenario](../../../tests/parts/cliharness/p018/body.rs) exercises the same fixture and timeout.

```sh
cargo run -- cluster harness-run \
  --fixture tests/fixtures/cluster-harness/two-node.cluster \
  --state-root target/cluster-walkthrough-state \
  --run-dir target/cluster-walkthrough-run \
  --child-timeout-ms 30000
```

The default child binary is the currently executing binary. No separate daemon installation is implied. The timeout is per child, admitted from 1 through 300000 milliseconds, not a promise about total workflow duration. Do not treat an unavailable build environment as a denied cluster experiment: if execution never starts, child evidence has not been produced.

## 3. Follow planning into child phases

The [runner entry](../../../src/cluster_harness/parts/runner/p000/body.rs) validates execution input, prepares roots, reads the fixture, and prepares three canonical artifacts: fixture metadata, command plan, and derived local process plan. The fixture ref binds fixture text in a dedicated domain. Logical handles such as `local-process:fixture-a` describe isolation in the plan; they are not live transport endpoints.

The shell then invokes node phases in manifest order. `init` initializes each node. `start` invokes the node run path. `workflow` invokes a bounded run loop with a maximum of one request and explicit workflow and heartbeat output files. `status` gathers status evidence. On the all-started path, `stop` runs in reverse order: fixture-b before fixture-a.

These are separate child invocations, not a concurrent two-node network exercise. The harness does not submit an application request merely by setting the run-loop bound. Later phases are gated on the previous phase's aggregate success.

## 4. Locate process and node observations

For each attempted phase, the shell writes a log under `logs/` and prepares a canonical process receipt under `children/processes/`. For example, the start observation for fixture-a is associated with `children/processes/start-fixture-a.preserves` and `logs/start-fixture-a.log`. The receipt binds the log ref and records exit, timeout, and orphan observations; the log is diagnostic text.

After the phases, [artifact capture](../../../src/cluster_harness/parts/runner/p003/body.rs) collects available node files under `children/receipts/fixture-a/` and the corresponding fixture-b directory. Configuration, identity, startup, workflow, heartbeat, health, status-control, shutdown, and stop-control artifacts are the complete successful capture set. Missing files are not manufactured from stdout.

Capture precedes ticket cleanup. Cleanup records observations; it does not erase the distinction between a successful child and one whose effects remain uncertain.

## 5. Read the parent and verify the directory

The shell builds cleanup, lifecycle, drift, local executable-run, and parent artifacts, then writes the sorted index and verification companion. A complete lifecycle requires all phases and the full per-node capture set. A denied final result may additionally produce a sealed diagnostic failure bundle.

This source-checked, unexecuted command uses the same [CLI declaration](../../../src/cli/ops/parts/cluster/p000/body.rs) and [offline verification scenario](../../../tests/parts/cliharness/p018/body.rs):

```sh
cargo run -- cluster harness-verify \
  --run-dir target/cluster-walkthrough-run
```

Verification reads the directory without starting children or rewriting its companion. Its useful result is an assessment of the indexed evidence, not permission to deploy.

## 6. Interpret an incomplete run honestly

The checked-in negative CLI scenario supplies a missing child binary. Its assertions expect command failure, a cleanup receipt, and sealed failure-bundle companions. This demonstrates the intended diagnostic export path in test source; it was not rerun here. Earlier filesystem or parsing failures can return before a complete export exists.

A second source-review boundary is partial startup: the runner attempts normal stop only when the whole start phase passes. Do not infer that every individually started node received a stop invocation from a cleanup summary alone. Preserve child evidence and investigate uncertainty. Even the successful path establishes local process integration only—not live transport, consensus, VM, or production readiness.

## Sources

- [Handbook](../README.md)
- [Governing receipt-first harness guide](../../receipt-first-cluster-harness.md)
- [Technical diagnosis companion](../../technical/operations/receipt-first-cluster-diagnosis.md)
- [Exact two-node fixture](../../../tests/fixtures/cluster-harness/two-node.cluster)
- [Runner phase sequencing](../../../src/cluster_harness/parts/runner/p000/body.rs)
- [Run finalization](../../../src/cluster_harness/parts/runner/p001/body.rs)
- [CLI positive and negative scenarios](../../../tests/parts/cliharness/p018/body.rs)
