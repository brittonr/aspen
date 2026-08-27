# Bounded DAG synchronization

Molten synchronizes graph metadata and referenced content through a receiver-owned plan. The protocol does not contain job, artifact, commit, or merge policy.

## Functional core

`molten-core::dag_sync` owns the deterministic graph laws. It validates roots, nodes, edges, cycles, bounds, strategies, responses, and resume progress.

Each plan binds these facts:

- the selected roots and schema references;
- the traversal epoch and service generation;
- the strategy and policy reference;
- the canonical peer inventory and assignments;
- the topological node order and missing objects.

Resume progress repeats the roots, schemas, peers, epoch, generation, strategy, and policy. Molten rejects progress when any fact changes.

The core supports full, stem-first, leaf-only, resumable, and peer-partitioned strategies. It does not select a strategy implicitly.

## Imperative shell

`molten::dag_sync` obtains authority and resource observations before each transfer. It then uses application-owned transport, content, progress, observation, and receipt ports.

The shell publishes a response observation before it stores progress. It publishes the completion receipt after all admitted progress records.

A partial run retains verified object references. A compatible restart omits those references from its new fetch plan.

Peer reassignment requires a new traversal epoch. Old progress cannot move to the new peer set.

## Conformance profiles

The deterministic profile uses `DeterministicTransportAdapter`. The live profile uses the existing Iroh loopback adapter through the fabric transport port.

Both profiles run the same DAG planner, response transition, progress transition, and domain-boundary code. DAG code does not import Iroh backend types.

The conformance tests cover these paths:

- complete deterministic and live runs;
- partial progress and restart;
- cancellation and partition;
- corruption and authority denial;
- peer reassignment;
- direct backend import denial.

## Operator status

The bounded status view shows roots, strategy, epoch, requests, verified objects, missing objects, peers, resources, failures, and evidence references.

The status view contains no payload bytes, credentials, transport handles, or mutable backend objects.

## Non-claims

A successful receipt proves verified availability for the requested references. It does not grant installation, execution, publication, merge, membership, provenance, or conflict authority.

Local completion does not prove global convergence. Peer assignment does not prove peer trust or availability.
