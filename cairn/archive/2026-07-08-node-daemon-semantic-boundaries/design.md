## Context

The node daemon owns the durable local node boundary. It currently acts as a broad implementation surface for lifecycle, ingress, dispatch, live workflow handoff, supervisor decisions, locks, and receipt emission.

## Design

### Proposed module ownership

- `config` and `identity`: node config, identity refs, startup inputs.
- `lock`: active/service lock state and stale-lock recovery decisions.
- `inbox`: queue paths, request identity, duplicate handling.
- `ingress`: local and live ingress preflight decisions.
- `dispatch`: operation routing and subreceipt binding.
- `supervision`: restart/shutdown policy and heartbeat decisions.
- `workflow`: live bundle import/apply/reconcile/ack planning.
- `receipts`: canonical node receipt constructors and parsers.
- `shell`: filesystem, live transport, and service-loop orchestration.

### Functional core boundary

Node admission, lock decisions, duplicate handling, supervisor decisions, and workflow protocol checks should be pure over typed inputs. Shell code owns state-root IO, live Iroh, control socket, and file writes.

### Compatibility

Existing `molten node` commands and root-crate module paths remain stable while internals move.

## Non-goals

- Do not redesign node-control protocol semantics.
- Do not change canonical receipt schemas or refs.
- Do not make live transport identity sufficient authority.
