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

Initial hostcalls are narrow declared refs around effect requests. This completed slice exercises declared `storage.read` and ambient `network.open` denial through plugin hostcall receipts. Broader send envelope, assert/retract/observe, blob/chunk get/put, typed storage write, trace emit, logical clock, and seeded random hostcalls require future explicit hostcall/effect manifest declarations and do not become ambient APIs.

## Lifecycle callbacks

Sandboxed artifacts may currently declare the supported initial callbacks:

- `init`,
- `start`,
- `health`,
- `stop`,
- `remove`,
- `upgrade`.

`artifact_info`, `handle_turn`, `handle_request`, `on_timer`, `on_event`, and richer shutdown surfaces are future ABI extensions. Callbacks run inside actor/service lifecycle and resource budgets. Callback failures are lifecycle events and can trigger supervision/cleanup receipts.

## Result/error encoding

Molten uses canonical Preserves `plugin-host-abi-result-v1` values rather than ad hoc strings. The implemented result shape carries ABI schema, `ok`/`error` status, optional payload ref, and explicit error text. Stable error class catalogs, redaction metadata, receipt refs, and retry/idempotency guidance should be represented by referenced receipts or a future admitted ABI extension rather than inferred from raw strings.

## Permissions and namespaces

Each execution gets an admitted effect manifest, namespace scopes, resource grants, supply-chain evidence, and authority context. Permission, lifecycle, and hostcall receipts validate declared hostcall refs, executor refs, effect refs, resources, and authority before doing anything effectful.

## Hot reload and upgrade

Hot reload is an upgrade-session task: install new artifact, validate ABI compatibility, drain or migrate state, switch metadata pointers, and emit receipts. It is not an arbitrary in-place plugin replacement.

## Open Questions

- When should a WIT/component wrapper be added around the current primitive Preserves/receipt ABI?
- Which future lifecycle exports should land first: `artifact_info`, `handle_turn`, `handle_request`, timer, or event callbacks?
- Should future ABI compatibility be structural, nominal, or both beyond the current plugin id, ABI, and retained schema checks?
