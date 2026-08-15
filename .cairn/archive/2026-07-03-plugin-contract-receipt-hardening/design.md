# Design: plugin contract receipt hardening

## Scope

This change strengthens the existing plugin host ABI and lifecycle contract without introducing new extension artifacts yet. It keeps the functional core as pure receipt/value validation and leaves filesystem access, registry reads, executor calls, and effect execution in the imperative shell.

## Binding model

Hostcall admission currently receives both an operation name and a hostcall ref. The pure core must treat the pair as a single contract claim:

```text
operation + declared hostcall descriptor -> expected hostcall ref
input hostcall ref == expected hostcall ref -> eligible for remaining gates
```

For the current primitive descriptors, `storage.read` and `network.open` derive from canonical `<plugin-hostcall OPERATION>` values. Future richer hostcall descriptors may include schema/effect metadata, but the same rule remains: the receipt must bind the operation to the descriptor ref it claims.

## Manifest identity

Every receipt that participates in lifecycle decisions should parse and expose the manifest ref it binds. Lifecycle evaluation should compare candidate receipts against the active manifest ref, not only the plugin identity. This prevents reusing a same-plugin receipt from an older artifact, stale manifest, or incompatible contract surface.

Required parsed bindings:

- install: plugin ref, manifest ref, artifact ref;
- permission: plugin ref, manifest ref;
- lifecycle: plugin ref, manifest ref, operation;
- hostcall: plugin ref, manifest ref, operation, hostcall ref;
- health: plugin ref, manifest ref;
- removal: plugin ref, manifest ref;
- upgrade: old manifest ref, new manifest ref;
- future compatibility/negotiation receipts: manifest ref plus contract refs.

## Check coherence

Receipt parsing must become verification-shaped, not just decoding-shaped. Required checks must be present with expected status. A `pass` decision is valid only when all required gates pass. A `deny` decision must carry at least one failed required gate or diagnostic explaining the denial.

This remains deterministic and local: no parser reaches out to registries, policies, executors, clocks, or files. It validates only the canonical receipt value and explicitly supplied active manifest state.

## Test strategy

Positive tests cover a declared hostcall where operation, descriptor ref, manifest ref, authority, resource, executor, and effect refs all match. Negative tests cover:

- operation/ref mismatch;
- missing manifest ref in a receipt shape that must carry one;
- stale manifest ref with the same plugin id;
- `decision=pass` with a required failed check;
- `decision=deny` without failed checks or diagnostics.

## Non-goals

- No new plugin extension contract artifact in this change.
- No runtime Nickel execution.
- No additional hostcall families beyond existing declared refs.
