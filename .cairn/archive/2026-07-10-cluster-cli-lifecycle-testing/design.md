# Design: cluster-cli-lifecycle-testing

## Overview

Extend CLI harness coverage around the existing `molten cluster` command surface. The test shell invokes the built binary through the existing CLI harness, while assertions inspect canonical artifacts under each node state root.

## Functional core and shell boundary

The functional core remains `molten::cluster::plan_cluster`, `render_cluster_manifest`, and `parse_cluster_manifest`. New test helpers may derive expected paths and receipt names from explicit inputs, but they must not hide filesystem mutation behind pure APIs.

The imperative shell owns temporary directories, command execution, stdout/stderr capture, and file existence checks.

## Positive coverage

A passing lifecycle test should initialize at least two nodes, start them, query status, stop them, and verify:

- `cluster.nodes` round-trips to the planned node ids;
- each node root contains config, identity, startup, health/control, heartbeat, shutdown, and control receipt evidence as applicable;
- `cluster start` recognizes already-running nodes without rewriting unrelated evidence;
- `cluster stop` handles nodes in reverse manifest order.

## Negative coverage

Negative fixtures should cover missing manifest, malformed manifest, empty manifest, unsafe node names, duplicate node ids, lifecycle collision, stale node lifecycle state, and non-forced reset denial. `--force` remains explicit destructive intent and should only remove planned node roots.

## Boundaries

Rendered stdout/stderr are diagnostic views. Passing CLI lifecycle evidence is local process evidence only and does not replace VM, live transport, consensus, authority, policy, provenance, or production evidence.
