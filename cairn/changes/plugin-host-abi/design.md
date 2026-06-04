## Context

Aspen's WASM host ABI documents concrete host functions, tagged result conventions, plugin lifecycle exports, permissions, namespace isolation, and resource limits. Molten should adopt the discipline, not the exact ABI. Molten's stable boundary is canonical Preserves plus effect/capability manifests.

## Goals

- Version every host ABI and record the version used by each execution.
- Expose only declared/admitted effects as hostcalls.
- Make result/error encoding canonical and language-neutral.
- Support lifecycle callbacks for init, ready/health, request/turn handling, timer/event callbacks, and shutdown.
- Enforce namespace isolation, resource limits, and capability checks at every hostcall.
- Emit receipts for lifecycle transitions and hostcall effect requests.

## Non-Goals

- Do not adopt Aspen's JSON/postcard RPC enum model.
- Do not expose ambient wall-clock, random, filesystem, network, or KV access.
- Do not let host ABI bypass Syndicate/SAM turn semantics or effect admission.
- Do not require one execution backend; ABI may target Wasmtime/component, Steel, or native adapters.

## ABI model

An ABI artifact should contain:

- ABI id and version,
- supported value encoding,
- hostcall set mapped to effect ids,
- guest exports/lifecycle callbacks,
- result/error encoding rules,
- resource and namespace constraints,
- compatibility notes,
- policy and evidence refs.

Initial hostcalls should be narrow wrappers around effect requests:

- send envelope / dataspace message,
- assert/retract/observe,
- blob/chunk get and put,
- typed storage read/write,
- trace emit,
- logical clock and seeded random when admitted.

## Lifecycle callbacks

Sandboxed artifacts may export callbacks such as:

- `artifact_info`,
- `init`,
- `handle_turn` or `handle_request`,
- `health`,
- `on_timer`,
- `on_event`,
- `shutdown`.

Callbacks run inside actor/service lifecycle and resource budgets. Callback failures are lifecycle events and can trigger supervision.

## Result/error encoding

Molten should use canonical Preserves result variants rather than ad hoc strings. Error values include stable error class, message, redaction metadata, receipt refs, and optional retry/idempotency guidance.

## Permissions and namespaces

Each execution gets an admitted effect manifest, namespace scopes, resource grants, and authority context. Hostcalls validate the call's effect id, schema, namespace, resource budget, and capability before doing anything effectful.

## Hot reload and upgrade

Hot reload is an upgrade-session task: install new artifact, validate ABI compatibility, drain or migrate state, switch metadata pointers, and emit receipts. It is not an arbitrary in-place plugin replacement.

## Open Questions

- Should first Wasm ABI use WIT/component model or a primitive Preserves byte interface?
- Which lifecycle exports are mandatory for the first milestone?
- Should ABI compatibility be structural, nominal, or both?
