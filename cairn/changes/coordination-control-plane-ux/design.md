## Context

`coordination-services-control-plane` defines the strongly consistent state machine and receipt model for locks, queues, semaphores, rate limits, elections, barriers, and service registry pointers. The remaining operator gap is ergonomic: the CLI only exposed a fixed fixture plus artifact summary.

## Goals

- Generate canonical coordination manifests without hand-writing Preserves records.
- Generate canonical coordination requests with explicit payload files and authority/policy/resource/operation-id refs.
- Apply a deterministic batch of request files through `apply_coordination_request`, never through ordinary actor messages or direct state edits.
- Emit a canonical `coordination-apply-report-v1` binding the manifest, final state, receipt refs, assertion refs, and evidence refs.
- Preserve duplicate operation-id semantics: applying the same request twice returns the prior semantic receipt and does not advance state a second time.
- Keep `show` read-only and useful for manifests, requests, receipts, fencing tokens, state snapshots, assertions, and apply reports.

## Non-Goals

- No long-running coordination daemon or persisted runtime state.
- No implicit authority, policy, resource, transport, provenance, or source-gate trust from CLI invocation.
- No actor-message, dataspace assertion, or local wall-clock shortcut for coordination mutation.
- No replacement for delivery idempotency receipts; operation ids remain explicit refs supplied by the caller.

## Implementation Notes

The CLI remains an imperative shell around the existing pure-ish coordination core. `manifest` and `request` call the canonical DTO builders and either write Preserves to `--out` or stdout. `apply` reads a manifest and ordered `--request` files, constructs an in-memory control-plane runtime, applies each request through the same admission/commit/idempotency path as tests, writes `report.preserves`, and indexes supporting evidence values. Denials are represented in normal coordination receipts and make the batch report decision `deny`.
