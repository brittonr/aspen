## Context

Molten already has remote artifact sync and ref-backed jobs. This change defines the common remote execution shape that can be used by node-control, job DAG workers, transcripts, and future actor endpoints.

## Design

### Execution request flow

```text
remote-execution-request-v1
  root artifact ref
  closure descriptor/ref
  entrypoint id
  canonical argument ref/value
  effect manifest ref
  requested handler profile
  presented capabilities
  policy/provenance/source-gate/resource evidence
  reply route
    -> receiver missing-set planning
    -> fetch and hash verification
    -> local install/admission
    -> handler binding
    -> execution receipt or deny receipt
```

The sender may provide hints and evidence refs, but the receiver computes the missing set and chooses whether to fetch, install, and execute.

### Closure descriptor

The closure descriptor binds root refs, expected direct deps, optional closure digest, artifact kinds, size/resource bounds, inline policy, handler profile constraints, and replay/session nonce. It does not authorize install by itself.

### Admission receipt

A passing admission receipt binds closure completeness, fetched refs, hash verification refs, local policy decision, provenance/source-gate evidence, capability attenuation, effect manifest/profile match, resource budget, and replay eligibility.

### No mobile closures

Requests may carry canonical arguments or content refs. They must not carry arbitrary live closures, heap snapshots, file descriptors, ambient environment, host process state, or unbounded serialized runtime state as executable authority.

### Functional core and shell

Pure cores validate request envelopes, compute missing sets from local summaries, check closure completeness, compare handler profiles, and decide denial reasons. Shells fetch over Iroh/blobs, write caches, invoke local gates, execute adapters, and persist receipts.

### Non-goals

- Do not adopt Unison Cloud, UCM remote execution protocol, bytecode format, or hash format.
- Do not let the sender force-install dependencies.
- Do not treat transport identity, gossip topic, ticket, or locator evidence as execution authority.
- Do not run incomplete or unadmitted dependency closures.